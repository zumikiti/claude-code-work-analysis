# Claude Work Analysis MCP Server

Claude Work Analysis ツールをMCP（Model Context Protocol）サーバーとして実装したものです。Claude Codeから直接リアルタイムで作業ログの分析が可能になります。

## 実装内容

### MCPツール一覧

1. **analyze_work_period** - Claude Code作業ログの期間分析
   - パラメータ: `from_date`, `to_date`, `project_filter`, `format`
   - 戻り値: 期間内の作業サマリー（Markdown/JSON形式）
   - 使用例: 「2025-06-01から2025-06-30の作業サマリーを出して」

2. **get_project_stats** - 特定プロジェクトの統計情報取得
   - パラメータ: `project_name`, `days`
   - 戻り値: プロジェクト固有の統計レポート
   - 使用例: 「[プロジェクト名]の過去30日の統計を教えて」

3. **summarize_recent** - 直近の作業活動サマリー
   - パラメータ: `days` (デフォルト7日)
   - 戻り値: 簡潔な最近の活動レポート
   - 使用例: 「今日の作業サマリーを時系列で出して」

### アーキテクチャの特徴

- **既存コード再利用**: `analyzer.rs`、`message_analyzer.rs`等をそのまま活用
- **トークン効率**: 生データではなく構造化サマリーを返却
- **リアルタイム性**: ファイルシステムから直接最新データを読み取り
- **JSON-RPC**: 標準的なMCPプロトコルに準拠

### ビルドと実行

```bash
# MCPサーバーのビルド
cargo build --bin mcp-server

# MCPサーバーの実行
./target/debug/mcp-server
```

### Claude Code統合

MCPサーバーとしてClaude Codeに統合するには、設定ファイル（`~/.claude/claude_desktop_config.json`）に以下を追加：

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

**設定後の確認:**
1. Claude Codeを再起動
2. `「今日の作業サマリーを時系列で出して」`などのプロンプトでテスト
3. MCPツールが正常に動作することを確認

### 利点

- **DuckDB不要**: シンプルな実装でトークン効率を維持
- **即時利用可能**: 複雑なDBセットアップが不要
- **高速レスポンス**: メモリ内処理によるパフォーマンス
- **既存機能保持**: コマンドライン版と同等の分析機能

## 実際の使用例

MCPサーバーを設定すると、Claude Codeで以下のようなプロンプトが使用できます：

```
# 今日の作業サマリーを時系列で出して
# 昨日の作業内容を詳しく教えて
# [プロジェクト名]の過去1週間の作業統計を出して
# 2025-06-01から2025-06-30の期間の作業分析をJSON形式で出して
```

これにより、Claude CodeからリアルタイムでClaude作業ログの分析が可能になります。