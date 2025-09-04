import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import './App.css';

interface AnalysisResults {
  total_files: number;
  total_groups: number;
  duplicate_results: {
    total_duplicates: number;
    space_wasted: number;
    duplicate_groups: Array<{
      base_name: string;
      duplicate_sets: string[][];
    }>;
  };
}

interface SearchResult {
  path: string;
  line_number?: number;
  snippet: string;
}

function App() {
  const [selectedPath, setSelectedPath] = useState<string>('');
  const [analysisResults, setAnalysisResults] = useState<AnalysisResults | null>(null);
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [isAnalyzing, setIsAnalyzing] = useState<boolean>(false);
  const [isSearching, setIsSearching] = useState<boolean>(false);
  const [error, setError] = useState<string>('');

  const selectDirectory = async () => {
    try {
      const selected = await open({
        directory: true,
        title: 'Select Directory to Analyze',
      });
      if (selected) {
        setSelectedPath(selected as string);
        setError('');
      }
    } catch (err) {
      setError(`Failed to select directory: ${err}`);
    }
  };

  const analyzeDirectory = async () => {
    if (!selectedPath) {
      setError('Please select a directory first');
      return;
    }

    setIsAnalyzing(true);
    setError('');
    try {
      const results = await invoke<AnalysisResults>('analyze_directory', {
        path: selectedPath,
      });
      setAnalysisResults(results);
    } catch (err) {
      setError(`Analysis failed: ${err}`);
    } finally {
      setIsAnalyzing(false);
    }
  };

  const searchFiles = async () => {
    if (!selectedPath || !searchQuery.trim()) {
      setError('Please select a directory and enter a search query');
      return;
    }

    setIsSearching(true);
    setError('');
    try {
      const results = await invoke<SearchResult[]>('search_files', {
        query: searchQuery,
        path: selectedPath,
        limit: 100,
      });
      setSearchResults(results);
    } catch (err) {
      setError(`Search failed: ${err}`);
    } finally {
      setIsSearching(false);
    }
  };

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };

  return (
    <div className="app">
      <header className="app-header">
        <h1>FERRET - File Analysis Tool</h1>
        <p>File Examination, Retrieval, and Redundancy Evaluation Tool</p>
      </header>

      <main className="app-main">
        {error && (
          <div className="error-message">
            {error}
          </div>
        )}

        <section className="directory-section">
          <h2>Directory Analysis</h2>
          <div className="directory-controls">
            <button onClick={selectDirectory} className="select-button">
              Select Directory
            </button>
            {selectedPath && (
              <div className="selected-path">
                <strong>Selected:</strong> {selectedPath}
              </div>
            )}
            <button 
              onClick={analyzeDirectory} 
              disabled={!selectedPath || isAnalyzing}
              className="analyze-button"
            >
              {isAnalyzing ? 'Analyzing...' : 'Analyze Directory'}
            </button>
          </div>
        </section>

        {analysisResults && (
          <section className="analysis-results">
            <h2>Analysis Results</h2>
            <div className="stats-grid">
              <div className="stat-card">
                <h3>Total Files</h3>
                <p>{analysisResults.total_files}</p>
              </div>
              <div className="stat-card">
                <h3>File Groups</h3>
                <p>{analysisResults.total_groups}</p>
              </div>
              <div className="stat-card">
                <h3>Duplicates Found</h3>
                <p>{analysisResults.duplicate_results.total_duplicates}</p>
              </div>
              <div className="stat-card">
                <h3>Space Wasted</h3>
                <p>{formatBytes(analysisResults.duplicate_results.space_wasted)}</p>
              </div>
            </div>

            {analysisResults.duplicate_results.duplicate_groups.length > 0 && (
              <div className="duplicate-groups">
                <h3>Duplicate Groups</h3>
                {analysisResults.duplicate_results.duplicate_groups.map((group, groupIndex) => (
                  <div key={groupIndex} className="duplicate-group">
                    <h4>{group.base_name}</h4>
                    {group.duplicate_sets.map((set, setIndex) => (
                      <div key={setIndex} className="duplicate-set">
                        <p><strong>Duplicate Set {setIndex + 1}:</strong></p>
                        <ul>
                          {set.map((file, fileIndex) => (
                            <li key={fileIndex}>{file}</li>
                          ))}
                        </ul>
                      </div>
                    ))}
                  </div>
                ))}
              </div>
            )}
          </section>
        )}

        <section className="search-section">
          <h2>File Search</h2>
          <div className="search-controls">
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Enter search query..."
              className="search-input"
            />
            <button 
              onClick={searchFiles} 
              disabled={!selectedPath || !searchQuery.trim() || isSearching}
              className="search-button"
            >
              {isSearching ? 'Searching...' : 'Search Files'}
            </button>
          </div>

          {searchResults.length > 0 && (
            <div className="search-results">
              <h3>Search Results ({searchResults.length})</h3>
              {searchResults.map((result, index) => (
                <div key={index} className="search-result">
                  <div className="result-header">
                    <strong>{result.path}</strong>
                    {result.line_number && (
                      <span className="line-number">Line {result.line_number}</span>
                    )}
                  </div>
                  <div className="result-snippet">
                    {result.snippet}
                  </div>
                </div>
              ))}
            </div>
          )}
        </section>
      </main>
    </div>
  );
}

export default App;
