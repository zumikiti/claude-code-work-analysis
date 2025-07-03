# Claude Work Analysis Rust

Claude Codeの作業ログを分析し、MCPサーバーとしてリアルタイムで統計情報を提供するRustツールです。

## 主な機能

- **MCPサーバー**: Claude Codeから直接リアルタイムで作業ログを分析
- **期間指定分析**: 特定の日付範囲での作業ログを分析
- **プロジェクト統計**: 特定プロジェクトに絞った詳細統計
- **セッション分析**: 作業セッションの自動検出と分類
- **JST対応**: 日本標準時での期間フィルタリング

## インストール

```bash
git clone https://github.com/your-username/claude-work-analysis-rust.git
cd claude-work-analysis-rust
cargo build --release
```

## 使用方法

### MCPサーバーとして利用（推奨）

MCPサーバーを起動してClaude Codeと連携する方法：

```bash
# MCPサーバーのビルド
cargo build --bin mcp-server

# MCPサーバーの起動
./target/debug/mcp-server
```

### Claude Code統合設定

Claude Codeの設定ファイル（`~/.claude/claude_desktop_config.json`）に以下を追加：

```json
{
  "mcpServers": {
    "claude-work-analysis": {
      "command": "/path/to/claude-work-analysis-rust/target/debug/mcp-server",
      "env": {}
    }
  }
}
```

### MCPツール一覧

1. **analyze_work_period** - 期間指定での作業分析
   - パラメータ: `from_date`, `to_date`, `project_filter`, `format`
   - 使用例: 「2025-06-01から2025-06-30の作業サマリーを出して」

2. **get_project_stats** - プロジェクト統計情報
   - パラメータ: `project_name`, `days`
   - 使用例: 「[プロジェクト名]の過去30日の統計を教えて」

3. **summarize_recent** - 直近の活動サマリー
   - パラメータ: `days` (デフォルト7日)
   - 使用例: 「今日の作業サマリーを時系列で出して」

## Claude Codeでの使用例

MCPサーバーを設定後、Claude Codeで以下のようなプロンプトが使用できます：

```
# 今日の作業サマリーを時系列で出して
# 昨日の作業内容を詳しく教えて
# [プロジェクト名]の過去1週間の作業統計を出して
# 2025-06-01から2025-06-30の期間の作業分析をJSON形式で出して
```

### コマンドライン版での利用

従来のコマンドライン版も利用可能です：

```bash
# 過去1週間の全プロジェクト分析
./target/release/claude-work-analysis --from 2025-06-23 --to 2025-06-30

# 特定プロジェクトの分析
./target/release/claude-work-analysis --project [プロジェクト名]

# JSON形式で出力
./target/release/claude-work-analysis --format json --output report.json
```

## アーキテクチャ

### データフロー
```
~/.claude/projects/ → ProjectScanner → JsonlParser → TimeRangeFilter → WorkAnalyzer → ReportGenerator → Output
```

### 主要コンポーネント

- **mcp_server.rs**: MCPサーバー実装（JSON-RPC準拠）
- **analyzer.rs**: セッション検出、活動分類、統計分析
- **parser.rs**: JSONL形式のClaudeログファイル解析
- **filter.rs**: 時間範囲・プロジェクト名によるフィルタリング（JST対応）
- **reporter.rs**: Markdown/JSON形式のレポート生成
- **models.rs**: データ構造定義（Claude対話ログ、分析結果等）

## 生成されるレポート内容

### 📊 Executive Summary
- 総作業セッション数、総メッセージ数、総作業時間
- 平均セッション長、アクティブプロジェクト数

### 🚀 Project Breakdown
- プロジェクト別の作業時間と統計
- 主要活動タイプ、セッション数とメッセージ数

### 🔍 Activity Analysis
- 活動タイプ別の時間配分
- コーディング、デバッグ、学習等の分類

### ⏰ Time Analysis
- 最も生産性の高い日、ピーク活動時間
- 日別活動サマリー

### 💬 Recent Sessions
- 最近の作業セッション詳細
- セッション期間とメッセージ数

## 開発

### ビルドとテスト
```bash
# 開発用ビルド
cargo build

# テスト実行
cargo test

# 型チェック
cargo check

# フォーマット
cargo fmt
```

### 設定とカスタマイズ
- セッション境界: 2時間以上の間隔で新セッション
- 最小メッセージ数: 3メッセージ以上で意味のあるセッション
- タイムゾーン: JST（日本標準時）で日付フィルタリング

## 利点

- **リアルタイム分析**: ファイルシステムから直接最新データを読み取り
- **トークン効率**: 生データではなく構造化サマリーを返却
- **シンプル**: DuckDBなど外部DBが不要
- **高速**: メモリ内処理によるパフォーマンス
- **JST対応**: 日本時間での期間指定が可能

## ライセンス

MIT License
