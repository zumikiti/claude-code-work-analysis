# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Claude Work Analysis Tool は、`~/.claude/projects/` のJSONLファイルを解析し、Claude Codeの作業ログから期間指定のサマリーレポートを生成するRustアプリケーションです。

## Build & Development Commands

```bash
# プロジェクトのビルド
cargo build --release

# 開発用ビルド
cargo build

# テスト実行
cargo test

# 型チェック
cargo check

# フォーマット
cargo fmt

# アプリケーション実行（リリースビルド）
./target/release/claude-work-analysis

# 期間指定での実行例
./target/release/claude-work-analysis --from 2025-06-23 --to 2025-06-30
```

## Core Architecture

### Data Flow Pipeline
```
~/.claude/projects/ → ProjectScanner → JsonlParser → TimeRangeFilter → WorkAnalyzer → ReportGenerator → Output
```

### Key Components

**models.rs** - データ構造の中核
- `ClaudeLogEntry`: Claude対話ログのJSONL構造
- `WorkSession`: 検出された作業セッション
- `WorkAnalysis`: 分析結果の統計情報
- `MessageContentVariant`: 文字列または構造化コンテンツ（画像等）を処理

**parser.rs** - JSONL解析エンジン
- `JsonlParser::parse_file()`: 非同期でJSONLファイルを解析
- 大容量ファイル対応（最大10MB/行）
- `skip_malformed: true`でエラー耐性を持つ
- Summary entryの自動スキップ機能

**analyzer.rs** - セッション分析の核心
- `WorkAnalyzer::analyze()`: メイン分析ロジック
- `session_gap_threshold: 2時間`でセッション境界を判定
- `MessageAnalyzer`統合による会話内容分析
- プロジェクト統計とトピック分析の生成

**message_analyzer.rs** - 会話内容分析（新機能）
- `analyze_session()`: セッション単位での技術・トピック抽出
- `analyze_conversations()`: 複数セッション横断の要約生成
- 日本語技術用語の認識（rust, typescript, react等）
- 問題解決パターンの抽出

## Data Model Structure

### Session Detection Logic
- **Gap Threshold**: 2時間以上の間隔で新セッション
- **Minimum Messages**: 3メッセージ以上で意味のあるセッション
- **Project Grouping**: `cwd`フィールドでプロジェクト分類

### Content Analysis
- **Technology Detection**: キーワードベースでの技術スタック抽出
- **Activity Classification**: User/Assistant比率による活動推定
- **Topic Extraction**: メッセージ内容からの主要トピック抽出

## Known Issues & Improvements

優先度の高い改善課題は`./claude/tasks/work-analysis-improvements.md`に記載：

1. **Parse Error Resolution** - JSONLパーサーの堅牢性（一部対応済み）
2. **Activity Classification** - "Other"が98.1%の分類精度問題
3. **Japanese NLP Support** - 日本語コンテンツ分析の改善が必要

## Configuration & Usage

### Command Line Arguments
- `--from DATE` / `--to DATE`: 分析期間（YYYY-MM-DD）
- `--project PROJECT`: 特定プロジェクトでフィルタリング
- `--output FILE`: 出力ファイルパス
- `--format FORMAT`: markdown（デフォルト） または json

### Default Behavior
引数なしで実行すると全期間・全プロジェクトを分析し、標準出力にMarkdown形式で結果を表示

## Development Notes

### Error Handling Strategy
- `anyhow::Result`でエラー伝播
- パーサーは`skip_malformed: true`でデータ損失を最小化
- 処理統計をログ出力（"Info: filename - Skipped X entries"）

### Performance Considerations
- 非同期ファイルI/O（tokio）
- 大容量JSONL対応（10MB/行まで）
- メモリ効率的な逐次処理

### Future Architecture Plans
- **DuckDB統合**（Issue #10）: 構造化データストレージ
- **MCP Server実装**（Issue #11）: Claude Codeとのリアルタイム統合

## Testing Strategy
- `tempfile`を使用した一時ファイルテスト
- モジュール単位でのunit test
- 統合テストはend-to-endのデータフロー検証