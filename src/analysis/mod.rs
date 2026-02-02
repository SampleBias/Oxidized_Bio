use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use csv::ReaderBuilder;
use nalgebra::{DMatrix, DVector};
use plotters::prelude::*;

use crate::data_registry::DatasetRecord;
use crate::models::{BiomarkerCandidate, DescriptiveStat, NoveltyScore, RegressionResult};

pub struct AnalysisConfig {
    pub target_column: Option<String>,
    pub group_column: Option<String>,
    pub covariates: Vec<String>,
    pub boxplot_column: Option<String>,
    pub max_columns: usize,
    pub max_groups: usize,
}

pub struct AnalysisArtifacts {
    pub descriptive_stats: Vec<DescriptiveStat>,
    pub regressions: Vec<RegressionResult>,
    pub novelty_scores: Vec<NoveltyScore>,
    pub biomarker_candidates: Vec<BiomarkerCandidate>,
    pub summary: String,
    pub heatmap_path: Option<String>,
    pub boxplot_path: Option<String>,
}

pub fn run_analysis(
    record: &DatasetRecord,
    config: &AnalysisConfig,
    output_dir: &Path,
) -> Result<AnalysisArtifacts> {
    let mut rdr = ReaderBuilder::new()
        .delimiter(record.delimiter)
        .has_headers(record.has_headers)
        .from_path(&record.local_path)
        .with_context(|| format!("Failed to open dataset {}", record.local_path))?;

    let headers: Vec<String> = if record.has_headers {
        rdr.headers()?
            .iter()
            .map(|h| h.to_string())
            .collect()
    } else {
        let mut cols = Vec::new();
        if let Some(first) = rdr.records().next() {
            let first = first?;
            for idx in 0..first.len() {
                cols.push(format!("column_{}", idx + 1));
            }
        }
        cols
    };

    let group_index = config
        .group_column
        .as_ref()
        .and_then(|c| headers.iter().position(|h| h == c));
    let target_index = config
        .target_column
        .as_ref()
        .and_then(|c| headers.iter().position(|h| h == c));

    let covariate_indices: Vec<(usize, String)> = config
        .covariates
        .iter()
        .filter_map(|c| headers.iter().position(|h| h == c).map(|idx| (idx, c.clone())))
        .collect();

    let boxplot_index = config
        .boxplot_column
        .as_ref()
        .and_then(|c| headers.iter().position(|h| h == c));

    let mut selected_indices: Vec<usize> = headers
        .iter()
        .enumerate()
        .filter(|(idx, _)| Some(*idx) != group_index)
        .map(|(idx, _)| idx)
        .take(config.max_columns)
        .collect();

    if selected_indices.is_empty() {
        selected_indices = headers.iter().enumerate().map(|(idx, _)| idx).collect();
    }

    let mut stats_values: Vec<Vec<f64>> = vec![Vec::new(); selected_indices.len()];
    let mut stats_min: Vec<f64> = vec![f64::INFINITY; selected_indices.len()];
    let mut stats_max: Vec<f64> = vec![f64::NEG_INFINITY; selected_indices.len()];

    let mut overall_sum: Vec<f64> = vec![0.0; selected_indices.len()];
    let mut overall_sum_sq: Vec<f64> = vec![0.0; selected_indices.len()];
    let mut overall_count: Vec<usize> = vec![0; selected_indices.len()];

    let mut group_sums: HashMap<String, Vec<(f64, usize)>> = HashMap::new();

    let mut regression_rows: Vec<Vec<f64>> = Vec::new();
    let mut regression_targets: Vec<f64> = Vec::new();
    let mut univariate_x: Vec<Vec<f64>> = vec![Vec::new(); selected_indices.len()];
    let mut univariate_y: Vec<Vec<f64>> = vec![Vec::new(); selected_indices.len()];
    let mut biomarker_x: Vec<Vec<f64>> = vec![Vec::new(); selected_indices.len()];
    let mut biomarker_y: Vec<Vec<f64>> = vec![Vec::new(); selected_indices.len()];

    let mut boxplot_values: HashMap<String, Vec<f64>> = HashMap::new();

    for record in rdr.records() {
        let record = record?;

        let group_value = group_index.and_then(|idx| record.get(idx).map(|v| v.to_string()));

        for (pos, col_idx) in selected_indices.iter().enumerate() {
            if let Some(val) = record.get(*col_idx) {
                if let Ok(parsed) = val.parse::<f64>() {
                    stats_values[pos].push(parsed);
                    stats_min[pos] = stats_min[pos].min(parsed);
                    stats_max[pos] = stats_max[pos].max(parsed);
                    overall_sum[pos] += parsed;
                    overall_sum_sq[pos] += parsed * parsed;
                    overall_count[pos] += 1;

                    if let Some(group_label) = &group_value {
                        let entry = group_sums
                            .entry(group_label.clone())
                            .or_insert_with(|| vec![(0.0, 0); selected_indices.len()]);
                        entry[pos].0 += parsed;
                        entry[pos].1 += 1;
                    }
                }
            }
        }

        if let Some(target_idx) = target_index {
            if let Some(target_val) = record.get(target_idx).and_then(|v| v.parse::<f64>().ok()) {
                for (pos, col_idx) in selected_indices.iter().enumerate() {
                    if *col_idx == target_idx {
                        continue;
                    }
                    if let Some(val) = record.get(*col_idx).and_then(|v| v.parse::<f64>().ok()) {
                        biomarker_x[pos].push(val);
                        biomarker_y[pos].push(target_val);
                    }
                }
                if covariate_indices.is_empty() {
                    for (pos, col_idx) in selected_indices.iter().enumerate() {
                        if let Some(val) = record.get(*col_idx).and_then(|v| v.parse::<f64>().ok()) {
                            univariate_x[pos].push(val);
                            univariate_y[pos].push(target_val);
                        }
                    }
                } else {
                    let mut row: Vec<f64> = Vec::with_capacity(covariate_indices.len());
                    let mut has_all = true;
                    for (idx, _) in &covariate_indices {
                        if let Some(val) = record.get(*idx).and_then(|v| v.parse::<f64>().ok()) {
                            row.push(val);
                        } else {
                            has_all = false;
                            break;
                        }
                    }
                    if has_all {
                        regression_rows.push(row);
                        regression_targets.push(target_val);
                    }
                }
            }
        }

        if let (Some(group_label), Some(box_idx)) = (&group_value, boxplot_index) {
            if let Some(val) = record.get(box_idx).and_then(|v| v.parse::<f64>().ok()) {
                boxplot_values.entry(group_label.clone()).or_default().push(val);
            }
        }
    }

    let descriptive_stats = build_descriptive_stats(&headers, &selected_indices, &stats_values, &stats_min, &stats_max)?;
    let regressions = if covariate_indices.is_empty() {
        build_univariate_regressions(
            config.target_column.as_ref(),
            &headers,
            &selected_indices,
            &univariate_x,
            &univariate_y,
        )?
    } else {
        build_regressions(
            config.target_column.as_ref(),
            &covariate_indices,
            &regression_rows,
            &regression_targets,
        )?
    };
    let novelty_scores = build_novelty_scores(
        &headers,
        &selected_indices,
        &overall_sum,
        &overall_sum_sq,
        &overall_count,
        &group_sums,
    );
    let biomarker_candidates = build_biomarker_candidates(
        config.target_column.as_ref(),
        &headers,
        &selected_indices,
        &biomarker_x,
        &biomarker_y,
    );

    let summary = format!(
        "Computed descriptive statistics for {} columns. Generated {} regression model(s). \
         Novelty scores computed for {} columns. Biomarker candidates ranked for {} columns.",
        descriptive_stats.len(),
        regressions.len(),
        novelty_scores.len(),
        biomarker_candidates.len()
    );

    let heatmap_path = if !stats_values.is_empty() {
        let path = output_dir.join("heatmap.png");
        let labels: Vec<String> = selected_indices
            .iter()
            .map(|idx| headers.get(*idx).cloned().unwrap_or_default())
            .collect();
        write_heatmap(&path, &stats_values, &labels)?;
        Some(path.to_string_lossy().to_string())
    } else {
        None
    };
    let boxplot_path = if !boxplot_values.is_empty() {
        let path = output_dir.join("boxplot.png");
        write_boxplot(&path, &boxplot_values, config.max_groups)?;
        Some(path.to_string_lossy().to_string())
    } else {
        None
    };

    Ok(AnalysisArtifacts {
        descriptive_stats,
        regressions,
        novelty_scores,
        biomarker_candidates,
        summary,
        heatmap_path,
        boxplot_path,
    })
}

fn build_descriptive_stats(
    headers: &[String],
    selected_indices: &[usize],
    values: &[Vec<f64>],
    mins: &[f64],
    maxes: &[f64],
) -> Result<Vec<DescriptiveStat>> {
    let mut stats = Vec::new();
    for (pos, col_idx) in selected_indices.iter().enumerate() {
        let mut col = values[pos].clone();
        if col.is_empty() {
            continue;
        }
        col.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let count = col.len();
        let mean = col.iter().sum::<f64>() / count as f64;
        let std_dev = std_dev(&col, mean);
        let median = if count % 2 == 0 {
            (col[count / 2 - 1] + col[count / 2]) / 2.0
        } else {
            col[count / 2]
        };
        stats.push(DescriptiveStat {
            column: headers.get(*col_idx).cloned().unwrap_or_else(|| format!("column_{}", col_idx + 1)),
            count,
            mean,
            std_dev,
            min: mins[pos],
            median,
            max: maxes[pos],
        });
    }
    Ok(stats)
}

fn std_dev(values: &[f64], mean: f64) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (values.len() as f64 - 1.0);
    variance.sqrt()
}

fn build_regressions(
    target: Option<&String>,
    covariates: &[(usize, String)],
    rows: &[Vec<f64>],
    targets: &[f64],
) -> Result<Vec<RegressionResult>> {
    let mut results = Vec::new();
    if let Some(target_name) = target {
        if !rows.is_empty() && rows.len() == targets.len() {
            let n = rows.len();
            let p = covariates.len();
            if p > 0 {
                let mut data = Vec::with_capacity(n * (p + 1));
                for row in rows {
                    data.push(1.0);
                    data.extend_from_slice(row);
                }
                let x = DMatrix::from_row_slice(n, p + 1, &data);
                let y = DVector::from_row_slice(targets);
                if let Some((intercept, coefficients, r2)) = ols_fit(&x, &y) {
                    results.push(RegressionResult {
                        target: target_name.clone(),
                        predictors: covariates.iter().map(|(_, name)| name.clone()).collect(),
                        intercept,
                        coefficients,
                        r2,
                        n,
                    });
                }
            }
        }
    }
    Ok(results)
}

fn build_univariate_regressions(
    target: Option<&String>,
    headers: &[String],
    selected_indices: &[usize],
    x_values: &[Vec<f64>],
    y_values: &[Vec<f64>],
) -> Result<Vec<RegressionResult>> {
    let mut results = Vec::new();
    if let Some(target_name) = target {
        for (pos, col_idx) in selected_indices.iter().enumerate() {
            if x_values[pos].len() < 2 || x_values[pos].len() != y_values[pos].len() {
                continue;
            }
            let n = x_values[pos].len();
            let mut data = Vec::with_capacity(n * 2);
            for val in &x_values[pos] {
                data.push(1.0);
                data.push(*val);
            }
            let x = DMatrix::from_row_slice(n, 2, &data);
            let y = DVector::from_row_slice(&y_values[pos]);
            if let Some((intercept, coefficients, r2)) = ols_fit(&x, &y) {
                results.push(RegressionResult {
                    target: target_name.clone(),
                    predictors: vec![headers.get(*col_idx).cloned().unwrap_or_else(|| format!("column_{}", col_idx + 1))],
                    intercept,
                    coefficients,
                    r2,
                    n,
                });
            }
        }
    }
    Ok(results)
}

fn ols_fit(x: &DMatrix<f64>, y: &DVector<f64>) -> Option<(f64, Vec<f64>, f64)> {
    let xtx = x.transpose() * x;
    let xtx_inv = xtx.try_inverse()?;
    let beta = xtx_inv * x.transpose() * y;
    let y_hat = x * &beta;
    let mean_y = y.iter().sum::<f64>() / y.len() as f64;
    let ss_tot = y.iter().map(|v| (v - mean_y).powi(2)).sum::<f64>();
    let ss_res = y
        .iter()
        .zip(y_hat.iter())
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>();
    let r2 = if ss_tot > 0.0 { 1.0 - (ss_res / ss_tot) } else { 0.0 };
    let intercept = beta.get(0).cloned().unwrap_or(0.0);
    let coefficients = beta.iter().skip(1).cloned().collect();
    Some((intercept, coefficients, r2))
}

fn build_novelty_scores(
    headers: &[String],
    selected_indices: &[usize],
    overall_sum: &[f64],
    overall_sum_sq: &[f64],
    overall_count: &[usize],
    group_sums: &HashMap<String, Vec<(f64, usize)>>,
) -> Vec<NoveltyScore> {
    let mut scores = Vec::new();
    for (pos, col_idx) in selected_indices.iter().enumerate() {
        if overall_count[pos] < 2 {
            continue;
        }
        let mean = overall_sum[pos] / overall_count[pos] as f64;
        let variance = (overall_sum_sq[pos] / overall_count[pos] as f64) - mean * mean;
        let std = variance.max(0.0).sqrt();
        let mut max_delta: f64 = 0.0;
        for (_group, sums) in group_sums {
            let (sum, count) = sums[pos];
            if count > 0 {
                let group_mean = sum / count as f64;
                max_delta = max_delta.max((group_mean - mean).abs());
            }
        }
        let score = if std > 0.0 { (max_delta / (3.0 * std)).min(1.0) } else { 0.0 };
        scores.push(NoveltyScore {
            column: headers.get(*col_idx).cloned().unwrap_or_else(|| format!("column_{}", col_idx + 1)),
            score,
            rationale: "Scaled deviation of group means from overall mean (0-1)".to_string(),
        });
    }
    scores
}

fn build_biomarker_candidates(
    target: Option<&String>,
    headers: &[String],
    selected_indices: &[usize],
    x_values: &[Vec<f64>],
    y_values: &[Vec<f64>],
) -> Vec<BiomarkerCandidate> {
    let mut candidates = Vec::new();
    if target.is_none() {
        return candidates;
    }

    for (pos, col_idx) in selected_indices.iter().enumerate() {
        if x_values[pos].len() < 3 || x_values[pos].len() != y_values[pos].len() {
            continue;
        }
        let corr = correlation(&x_values[pos], &y_values[pos]);
        let score = corr.abs();
        let direction = if corr >= 0.0 { "positive" } else { "negative" };
        candidates.push(BiomarkerCandidate {
            column: headers.get(*col_idx).cloned().unwrap_or_else(|| format!("column_{}", col_idx + 1)),
            score,
            correlation: corr,
            direction: direction.to_string(),
            notes: "Pearson correlation with target (age). Higher absolute correlation suggests stronger biomarker signal.".to_string(),
        });
    }

    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(50);
    candidates
}

pub fn write_heatmap(
    output_path: &Path,
    stats_values: &[Vec<f64>],
    labels: &[String],
) -> Result<()> {
    let size = stats_values.len().min(20);
    if size == 0 {
        return Ok(());
    }
    let mut corr = vec![vec![0.0; size]; size];
    for i in 0..size {
        for j in 0..size {
            corr[i][j] = correlation(&stats_values[i], &stats_values[j]);
        }
    }

    let root = BitMapBackend::new(output_path, (800, 800)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .margin(20)
        .caption("Correlation Heatmap", ("sans-serif", 24))
        .build_cartesian_2d(0..size, 0..size)?;

    chart
        .configure_mesh()
        .disable_mesh()
        .x_labels(size)
        .y_labels(size)
        .x_label_formatter(&|x| labels.get(*x).cloned().unwrap_or_default())
        .y_label_formatter(&|y| labels.get(*y).cloned().unwrap_or_default())
        .draw()?;

    for i in 0..size {
        for j in 0..size {
            let val = corr[i][j];
            let color = HSLColor(240.0 / 360.0 - (240.0 / 360.0) * ((val + 1.0) / 2.0), 0.7, 0.5);
            chart.draw_series(std::iter::once(Rectangle::new(
                [(i, j), (i + 1, j + 1)],
                color.filled(),
            )))?;
        }
    }

    Ok(())
}

pub fn write_boxplot(
    output_path: &Path,
    grouped: &HashMap<String, Vec<f64>>,
    max_groups: usize,
) -> Result<()> {
    if grouped.is_empty() {
        return Ok(());
    }
    let mut groups: Vec<(String, Vec<f64>)> = grouped
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    groups.sort_by(|a, b| a.0.cmp(&b.0));
    groups.truncate(max_groups);

    let mut global_min = f64::INFINITY;
    let mut global_max = f64::NEG_INFINITY;
    let mut stats = Vec::new();
    for (_label, values) in &groups {
        let mut v = values.clone();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        if v.is_empty() {
            continue;
        }
        global_min = global_min.min(*v.first().unwrap());
        global_max = global_max.max(*v.last().unwrap());
        let q1 = v[v.len() / 4];
        let median = v[v.len() / 2];
        let q3 = v[(v.len() * 3) / 4];
        let min = *v.first().unwrap();
        let max = *v.last().unwrap();
        stats.push((q1, median, q3, min, max));
    }

    let root = BitMapBackend::new(output_path, (900, 500)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .margin(20)
        .caption("Box Plot by Group", ("sans-serif", 24))
        .build_cartesian_2d(0f64..groups.len() as f64, global_min..global_max)?;

    chart
        .configure_mesh()
        .disable_mesh()
        .x_labels(groups.len())
        .x_label_formatter(&|x| {
            let idx = (*x).floor() as usize;
            groups.get(idx).map(|g| g.0.clone()).unwrap_or_default()
        })
        .draw()?;

    for (idx, stat) in stats.iter().enumerate() {
        let (q1, median, q3, min, max) = *stat;
        let idx_f = idx as f64;
        let rect = Rectangle::new([(idx_f, q1), (idx_f + 1.0, q3)], BLUE.mix(0.3).filled());
        chart.draw_series(std::iter::once(rect))?;
        chart.draw_series(std::iter::once(PathElement::new(vec![(idx_f, median), (idx_f + 1.0, median)], &BLUE)))?;
        chart.draw_series(std::iter::once(PathElement::new(vec![(idx_f + 0.5, q3), (idx_f + 0.5, max)], &BLACK)))?;
        chart.draw_series(std::iter::once(PathElement::new(vec![(idx_f + 0.5, q1), (idx_f + 0.5, min)], &BLACK)))?;
    }
    Ok(())
}

pub fn build_manuscript(
    dataset_id: &str,
    target: &str,
    group: &str,
    record: &crate::data_registry::DatasetRecord,
    analysis: &AnalysisArtifacts,
) -> String {
    let project_id = format!("OXBIO-{}", dataset_id);
    let top_biomarkers: Vec<String> = analysis
        .biomarker_candidates
        .iter()
        .take(10)
        .map(|b| format!("{} (r={:.3}, {})", b.column, b.correlation, b.direction))
        .collect();
    let top_list = if top_biomarkers.is_empty() {
        "No biomarker candidates were identified.".to_string()
    } else {
        top_biomarkers.join(", ")
    };

    format!(
        "Project ID: {project_id}\n\
Title: Biomarker discovery in log2-normalized microarray data\n\
\n\
Abstract\n\
We analyzed log2-normalized microarray data to identify aging-associated biomarkers. \
The dataset contained {rows} rows and {cols} columns. Using descriptive statistics, \
regression modeling, and biomarker ranking by correlation with {target}, we identified \
candidate biomarkers with the strongest association to aging.\n\
\n\
Methods\n\
Data ingestion validated CSV/TSV structure and inferred column headers. \
Descriptive statistics were computed per numeric marker. Linear regression models were fit \
to explain {target} from specified covariates. Biomarker candidates were ranked by Pearson \
correlation with {target}. Group-level distributions were summarized by {group}. \
Correlation heatmaps and box plots were generated for exploratory analysis.\n\
\n\
Results\n\
Computed descriptive statistics for {stat_count} markers, regressions for {reg_count} model(s), \
and novelty scores for {novelty_count} markers. Top biomarker candidates: {top_list}.\n\
\n\
Discussion\n\
Markers with strong correlations to {target} represent candidate aging biomarkers in this \
dataset. These findings provide a ranked shortlist for downstream validation (e.g., \
replication cohorts, pathway analysis, or mechanistic experiments). \
Because the data are already log2-normalized, relative effect sizes are interpretable in \
log2 space. The correlation-based ranking provides a fast triage; additional modeling \
and multiple-testing correction are recommended for definitive claims.\n\
\n\
Limitations\n\
The analysis assumes numeric columns are properly normalized and does not perform batch \
correction or probe re-annotation. Statistical significance testing and biological \
annotation are not yet included.\n",
        rows = record.row_count,
        cols = record.columns.len(),
        target = target,
        group = group,
        stat_count = analysis.descriptive_stats.len(),
        reg_count = analysis.regressions.len(),
        novelty_count = analysis.novelty_scores.len(),
        top_list = top_list
    )
}

fn correlation(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 {
        return 0.0;
    }
    let mean_x = x.iter().take(n).sum::<f64>() / n as f64;
    let mean_y = y.iter().take(n).sum::<f64>() / n as f64;
    let mut num = 0.0;
    let mut denom_x = 0.0;
    let mut denom_y = 0.0;
    for i in 0..n {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        num += dx * dy;
        denom_x += dx * dx;
        denom_y += dy * dy;
    }
    if denom_x == 0.0 || denom_y == 0.0 {
        0.0
    } else {
        num / (denom_x.sqrt() * denom_y.sqrt())
    }
}
