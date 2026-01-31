import { route } from 'preact-router';

export function Header({ showSettings = true }) {
  return (
    <div className="header">
      <div className="header-left">
        <div className="header-title" onClick={() => route('/chat')}>
          BioAgents
        </div>
      </div>
      {showSettings && (
        <div className="header-right">
          <button 
            className="settings-button" 
            onClick={() => route('/settings')}
            title="Settings"
          >
            <svg 
              width="20" 
              height="20" 
              viewBox="0 0 24 24" 
              fill="none" 
              stroke="currentColor" 
              strokeWidth="2" 
              strokeLinecap="round" 
              strokeLinejoin="round"
            >
              <circle cx="12" cy="12" r="3"></circle>
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
            </svg>
          </button>
        </div>
      )}
      <style>{`
        .header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 12px 20px;
          background: var(--bg-secondary, #1a1a1a);
          border-bottom: 1px solid var(--border-color, #333);
        }
        .header-left {
          display: flex;
          align-items: center;
        }
        .header-title {
          font-size: 1.25rem;
          font-weight: 600;
          color: var(--text-primary, #fff);
          cursor: pointer;
        }
        .header-title:hover {
          color: var(--accent-color, #3b82f6);
        }
        .header-right {
          display: flex;
          align-items: center;
          gap: 12px;
        }
        .settings-button {
          display: flex;
          align-items: center;
          justify-content: center;
          width: 36px;
          height: 36px;
          background: transparent;
          border: 1px solid var(--border-color, #333);
          border-radius: 8px;
          color: var(--text-secondary, #a1a1a1);
          cursor: pointer;
          transition: all 0.2s ease;
        }
        .settings-button:hover {
          background: var(--bg-tertiary, #2a2a2a);
          color: var(--text-primary, #fff);
          border-color: var(--accent-color, #3b82f6);
        }
      `}</style>
    </div>
  );
}
