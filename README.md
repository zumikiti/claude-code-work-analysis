# Claude Work Analysis Tool

`~/.claude/projects/` の内容を期間指定して取得し、Claude Codeに読ませて作業内容の週次サマリーを生成するRustツール。

## 機能

- **期間指定分析**: 特定の日付範囲での作業ログを分析
- **プロジェクトフィルタリング**: 特定プロジェクトに絞った分析
- **セッション分析**: 作業セッションの自動検出と分類
- **活動分類**: コーディング、デバッグ、学習等の活動自動分類
- **レポート生成**: MarkdownまたはJSON形式での詳細レポート

## インストール

```bash
git clone <repository-url>
cd claude-work-analysis-rust
cargo build --release
```

## 使用方法

### 基本的な使用例

```bash
# 過去1週間の全プロジェクト分析
./target/release/claude-work-analysis --from 2025-06-23 --to 2025-06-30

# 特定プロジェクトの分析
./target/release/claude-work-analysis --project [プロジェクト名]

# JSON形式で出力
./target/release/claude-work-analysis --format json

# ファイルに出力
./target/release/claude-work-analysis --output weekly-report.md
```

### コマンドライン引数

- `--from DATE`: 分析開始日（YYYY-MM-DD形式）
- `--to DATE`: 分析終了日（YYYY-MM-DD形式）
- `--project PROJECT`: 特定プロジェクト名でフィルタリング
- `--output FILE`: 出力ファイルパス
- `--format FORMAT`: 出力形式（markdown または json、デフォルト: markdown）

## アーキテクチャ

### データフロー
```
~/.claude/projects/ 
  → ProjectScanner 
  → JsonlParser 
  → TimeRangeFilter 
  → WorkAnalyzer 
  → ReportGenerator 
  → Output (Markdown/JSON)
```

### 主要コンポーネント

- **models.rs**: データ構造定義（Claude対話ログ、分析結果等）
- **scanner.rs**: `~/.claude/projects/` の走査とJSONLファイル検出
- **parser.rs**: JSONL形式のClaudeログファイル解析
- **filter.rs**: 時間範囲・プロジェクト名によるフィルタリング
- **analyzer.rs**: セッション検出、活動分類、統計分析
- **reporter.rs**: Markdown/JSON形式のレポート生成

## レポート内容

生成されるレポートには以下の情報が含まれます：

### 📊 Executive Summary
- 総作業セッション数
- 総メッセージ数
- 総作業時間
- 平均セッション長
- アクティブプロジェクト数

### 🚀 Project Breakdown
- プロジェクト別の作業時間と統計
- 主要活動タイプ
- セッション数とメッセージ数

### 🔍 Activity Analysis
- 活動タイプ別の時間配分
- コーディング、デバッグ、学習等の分類

### ⏰ Time Analysis
- 最も生産性の高い日
- ピーク活動時間
- 日別活動サマリー

### 💬 Recent Sessions
- 最近の作業セッション詳細
- セッション期間とメッセージ数

### 💡 Insights & Recommendations
- 作業パターンに基づく洞察
- 生産性向上のための提案

## Claude Code連携

生成されたレポートをClaude Codeで読み込んで、さらに詳細な分析や質問を行うことができます：

```bash
# レポートを生成
./claude-work-analysis --from 2025-06-01 --output work-summary.md

# Claude Codeでレポートを読み込み
claude read work-summary.md
```

## 開発

### テスト実行
```bash
cargo test
```

### 型チェック
```bash
cargo check
```

### フォーマット
```bash
cargo fmt
```

## ライセンス

MIT License

## 貢献

Issue報告やPull Requestを歓迎します。
