# FERRET Implementation Issues

This document tracks critical issues that need resolution to complete the implementation according to the architecture guide.

## Critical Issues Requiring Resolution

### 1. **Tauri Desktop Application Build Failure**
**Status**: ❌ BLOCKING
**Issue**: Tauri application fails to build due to missing icon file requirement
```
`icons/icon.ico` not found; required for generating a Windows Resource file during tauri-build
```
**Impact**: Desktop GUI cannot be built or tested
**Solution Needed**: Create proper icon files or configure Tauri to not require them

### 2. **Search Engine Functionality Incomplete**
**Status**: ❌ MAJOR
**Issue**: Search functionality is not working as a proper search engine
- Current implementation only does basic ripgrep searches
- No indexing for fast search results
- No search result ranking or relevance scoring
- Missing content extraction for searchable text

**Expected Behavior**: Should work like a desktop search engine with:
- Indexed content for fast retrieval
- Relevance scoring and ranking
- Content previews and snippets
- Search across file contents, not just filenames

### 3. **Unused Code and Dead Dependencies**
**Status**: ⚠️ TECHNICAL DEBT
**Issue**: Large amount of unused code indicates incomplete implementation
- 28+ compiler warnings for unused code
- Many structs and methods never used (NetworkAwareIO, RetryManager, DocumentExtractor, etc.)
- Suggests features are partially implemented but not integrated

**Impact**: Code bloat, maintenance burden, unclear what's actually working

### 4. **Missing Core Search Engine Features**
**Status**: ❌ MAJOR
**Issue**: The tool should function as a search engine but lacks key features:
- No persistent index storage
- No incremental indexing
- No search result caching
- No advanced search operators
- No search history or suggestions

### 5. **Document Extraction Not Working**
**Status**: ❌ MAJOR  
**Issue**: Document extraction methods are implemented but never used
- PDF, DOCX, XLSX extraction code exists but unused
- Search can't find content inside documents
- Limits search to plain text files only

## Questions for Resolution

1. **Icon Requirements**: Should we create proper icon files for Tauri or disable the requirement?
2. **Search Engine Scope**: What level of search engine functionality is expected? (basic file search vs. full desktop search engine)
3. **Dead Code**: Should unused code be removed or are these features planned for future implementation?
4. **Indexing Strategy**: Should we implement persistent indexing or keep the current JSON-based approach?
5. **Document Processing**: Should document extraction be integrated into the search functionality?

## Next Steps

1. Fix Tauri build issue to enable desktop application testing
2. Define search engine requirements and implement accordingly  
3. Clean up unused code or integrate missing features
4. Test complete workflow from indexing to searching
5. Verify desktop application functionality