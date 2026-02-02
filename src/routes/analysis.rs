use std::path::Path;

use axum::{extract::State, routing::post, Json, Router};
use tokio::fs;
use tracing::info;

use crate::analysis::{AnalysisConfig, run_analysis};
use crate::models::{AnalysisRequest, AnalysisResponse, AppState, AnalysisArtifact};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/analysis", post(run_analysis_handler))
        .with_state(state)
}

async fn run_analysis_handler(
    State(state): State<AppState>,
    Json(request): Json<AnalysisRequest>,
) -> Result<Json<AnalysisResponse>, axum::http::StatusCode> {
    info!(dataset_id = %request.dataset_id, "Analysis request received");

    let record = state
        .dataset_registry
        .get(&request.dataset_id)
        .await
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;

    let output_dir = Path::new("artifacts")
        .join("analysis")
        .join(&request.dataset_id);
    fs::create_dir_all(&output_dir)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    let config = AnalysisConfig {
        target_column: request.target_column.clone(),
        group_column: request.group_column.clone(),
        covariates: request.covariates.clone().unwrap_or_default(),
        boxplot_column: request.boxplot_column.clone(),
        max_columns: request.max_columns.unwrap_or(50),
        max_groups: request.max_groups.unwrap_or(20),
    };

    let analysis = run_analysis(&record, &config, &output_dir)
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut artifacts: Vec<AnalysisArtifact> = Vec::new();
    let stats_path = output_dir.join("descriptive_stats.csv");
    let regression_path = output_dir.join("regressions.csv");
    let novelty_path = output_dir.join("novelty_scores.csv");

    write_stats_csv(&stats_path, &analysis.descriptive_stats)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    write_regression_csv(&regression_path, &analysis.regressions)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    write_novelty_csv(&novelty_path, &analysis.novelty_scores)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    artifacts.push(AnalysisArtifact {
        id: "descriptive_stats".to_string(),
        description: "Descriptive statistics per numeric column".to_string(),
        artifact_type: "FILE".to_string(),
        content: None,
        name: "descriptive_stats.csv".to_string(),
        path: Some(stats_path.to_string_lossy().to_string()),
    });
    artifacts.push(AnalysisArtifact {
        id: "regressions".to_string(),
        description: "Linear regression results".to_string(),
        artifact_type: "FILE".to_string(),
        content: None,
        name: "regressions.csv".to_string(),
        path: Some(regression_path.to_string_lossy().to_string()),
    });
    artifacts.push(AnalysisArtifact {
        id: "novelty_scores".to_string(),
        description: "Novelty scoring based on group mean deviation".to_string(),
        artifact_type: "FILE".to_string(),
        content: None,
        name: "novelty_scores.csv".to_string(),
        path: Some(novelty_path.to_string_lossy().to_string()),
    });

    if let Some(path) = analysis.heatmap_path.clone() {
        artifacts.push(AnalysisArtifact {
            id: "heatmap".to_string(),
            description: "Correlation heatmap".to_string(),
            artifact_type: "FILE".to_string(),
            content: None,
            name: "heatmap.png".to_string(),
            path: Some(path),
        });
    }
    if let Some(path) = analysis.boxplot_path.clone() {
        artifacts.push(AnalysisArtifact {
            id: "boxplot".to_string(),
            description: "Box plot by group".to_string(),
            artifact_type: "FILE".to_string(),
            content: None,
            name: "boxplot.png".to_string(),
            path: Some(path),
        });
    }

    let response = AnalysisResponse {
        status: "success".to_string(),
        dataset_id: request.dataset_id,
        summary: analysis.summary,
        descriptive_stats: analysis.descriptive_stats,
        regressions: analysis.regressions,
        novelty_scores: analysis.novelty_scores,
        artifacts,
    };

    Ok(Json(response))
}

async fn write_stats_csv(path: &Path, stats: &[crate::models::DescriptiveStat]) -> anyhow::Result<()> {
    let mut wtr = csv::Writer::from_writer(Vec::new());
    wtr.write_record(["column", "count", "mean", "std_dev", "min", "median", "max"])?;
    for stat in stats {
        wtr.write_record([
            &stat.column,
            &stat.count.to_string(),
            &stat.mean.to_string(),
            &stat.std_dev.to_string(),
            &stat.min.to_string(),
            &stat.median.to_string(),
            &stat.max.to_string(),
        ])?;
    }
    let data = wtr.into_inner()?;
    fs::write(path, data).await?;
    Ok(())
}

async fn write_regression_csv(path: &Path, regressions: &[crate::models::RegressionResult]) -> anyhow::Result<()> {
    let mut wtr = csv::Writer::from_writer(Vec::new());
    wtr.write_record(["target", "predictors", "intercept", "coefficients", "r2", "n"])?;
    for reg in regressions {
        wtr.write_record([
            &reg.target,
            &reg.predictors.join(";"),
            &reg.intercept.to_string(),
            &reg.coefficients.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(";"),
            &reg.r2.to_string(),
            &reg.n.to_string(),
        ])?;
    }
    let data = wtr.into_inner()?;
    fs::write(path, data).await?;
    Ok(())
}

async fn write_novelty_csv(path: &Path, novelty: &[crate::models::NoveltyScore]) -> anyhow::Result<()> {
    let mut wtr = csv::Writer::from_writer(Vec::new());
    wtr.write_record(["column", "score", "rationale"])?;
    for score in novelty {
        wtr.write_record([&score.column, &score.score.to_string(), &score.rationale])?;
    }
    let data = wtr.into_inner()?;
    fs::write(path, data).await?;
    Ok(())
}
