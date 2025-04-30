# Model Context Protocol (MCP) - Library Specification

## 目的と概要

MCP（Model Context Protocol）は、AI assistantが外部ソースコードを効率的に検索、分析、参照するためのツールセットを提供します。このプロトコルにより、AIは以下のことが可能になります：

- GitHub上の関連リポジトリの検索
- 特定リポジトリのコードグレップによる詳細分析
- リポジトリのブランチとタグの閲覧

主なユースケース：

- コード例やパターンの検索
- 特定の実装方法の調査
- ライブラリやフレームワークの使用方法の理解
- バージョン間の違いの分析

## 基本設計と共通機能

### プロセスシード

MCPはstartしたときにランダムなseed値をオンメモリに保持します。このseed値は並行してこのMCPが実行された際にlocal directory pathがコンフリクトを避けるために使用されます。
このseedを`process seed`と呼びます。

```rust
use rand::{thread_rng, Rng};
use std::sync::atomic::{AtomicU64, Ordering};

// プロセス全体で共有されるシード値
static PROCESS_SEED: AtomicU64 = AtomicU64::new(0);

fn initialize_process_seed() {
    let seed = thread_rng().gen::<u64>();
    PROCESS_SEED.store(seed, Ordering::SeqCst);
}

fn get_process_seed() -> u64 {
    let current = PROCESS_SEED.load(Ordering::SeqCst);
    if current == 0 {
        initialize_process_seed();
        PROCESS_SEED.load(Ordering::SeqCst)
    } else {
        current
    }
}
```

### 標準レスポンス形式

すべてのツールは統一されたレスポンス形式に従います：

```rust
pub struct ToolResponse<T> {
    // 操作が成功したかどうか
    pub success: bool,
    // 結果データ（ツール固有の型）
    pub data: Option<T>,
    // エラー情報（失敗した場合）
    pub error: Option<ErrorInfo>,
    // メタデータ（実行時間、使用したリソースなど）
    pub metadata: ResponseMetadata,
}

pub struct ErrorInfo {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

pub struct ResponseMetadata {
    pub execution_time_ms: u64,
    pub rate_limit_remaining: Option<u32>,
    pub rate_limit_reset: Option<u64>,
}
```

### CallToolResult型

`CallToolResult`は、ツール呼び出しの結果をラップする標準型です：

```rust
pub enum CallToolResult {
    // 成功したツール呼び出し
    Success(ToolResponse<serde_json::Value>),
    // エラーが発生したツール呼び出し
    Failure {
        error_type: ErrorType,
        message: String,
        details: Option<serde_json::Value>,
    },
}

pub enum ErrorType {
    // リクエスト形式に問題がある
    InvalidRequest,
    // API連携の問題
    ApiError,
    // リソースが見つからない
    NotFound,
    // 権限不足
    Forbidden,
    // レート制限到達
    RateLimited,
    // 内部サーバーエラー
    InternalError,
}
```

## 提供するツール

### 1. GitHub リポジトリ検索ツール

GitHub APIを使用してリポジトリを検索します。

#### 入力パラメータ

```rust
pub struct SearchRepositoriesRequest {
    // 検索クエリ（必須）
    pub query: String,
    // 結果の並べ替え方法（オプション、デフォルトは「関連性」）
    pub sort_by: Option<SortBy>,
    // 並べ替えの順序（オプション、デフォルトは「降順」）
    pub order: Option<SortOrder>,
    // 1ページあたりの結果数（オプション、デフォルトは30、最大100）
    pub per_page: Option<u8>,
    // 結果のページ番号（オプション、デフォルトは1）
    pub page: Option<u32>,
}

pub enum SortBy {
    Relevance,
    Stars,
    Forks,
    Updated,
}

pub enum SortOrder {
    Ascending,
    Descending,
}
```

#### 実装詳細

- API エンドポイント: `https://api.github.com/search/repositories?q={query}`
- リファレンスドキュメント: https://docs.github.com/en/rest/search/search

#### API認証

- 環境変数 `GITCODE_MCP_GITHUB_TOKEN` で個人アクセストークンを提供。
- トークンが提供されない場合は、非認証リクエストを使用（レート制限あり）
- 非認証リクエスト: 60リクエスト/時
- 認証済みリクエスト: 5,000リクエスト/時

> **注意**: プライベートリポジトリへのアクセスには、適切な権限を持つアクセストークンが必要です。トークンには最低限、`repo` スコープ（プライベートリポジトリにアクセスする権限）が必要です。

#### 戻り値

```rust
pub struct SearchRepositoriesResult {
    // 検索結果のリポジトリリスト
    pub repositories: Vec<Repository>,
    // 検索結果の総数
    pub total_count: u32,
    // 現在のページ番号
    pub page: u32,
    // 1ページあたりの結果数
    pub per_page: u8,
}

pub struct Repository {
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub html_url: String,
    pub description: Option<String>,
    pub fork: bool,
    pub created_at: String,
    pub updated_at: String,
    pub pushed_at: String,
    pub git_url: String,
    pub ssh_url: String,
    pub clone_url: String,
    pub svn_url: String,
    pub homepage: Option<String>,
    pub language: Option<String>,
    pub license: Option<License>,
    pub topics: Vec<String>,
    pub visibility: String,
    pub default_branch: String,
}

pub struct License {
    pub key: String,
    pub name: String,
    pub spdx_id: String,
    pub url: Option<String>,
}
```

#### レスポンス例

```json
{
  "success": true,
  "data": {
    "repositories": [
      {
        "name": "llm",
        "full_name": "simonw/llm",
        "private": false,
        "html_url": "https://github.com/simonw/llm",
        "description": "Access large language models from the command-line",
        "fork": false,
        "created_at": "2023-04-01T21:16:57Z",
        "updated_at": "2025-04-30T14:24:55Z",
        "pushed_at": "2025-04-23T17:55:27Z",
        "git_url": "git://github.com/simonw/llm.git",
        "ssh_url": "git@github.com:simonw/llm.git",
        "clone_url": "https://github.com/simonw/llm.git",
        "svn_url": "https://github.com/simonw/llm",
        "homepage": "https://llm.datasette.io",
        "language": "Python",
        "license": {
          "key": "apache-2.0",
          "name": "Apache License 2.0",
          "spdx_id": "Apache-2.0",
          "url": "https://api.github.com/licenses/apache-2.0"
        },
        "topics": ["ai", "llms", "openai"],
        "visibility": "public",
        "default_branch": "main"
      }
    ],
    "total_count": 145,
    "page": 1,
    "per_page": 30
  },
  "metadata": {
    "execution_time_ms": 234,
    "rate_limit_remaining": 4998,
    "rate_limit_reset": 1620000000
  }
}
```

### 2. GitHub リポジトリコードGrep ツール

指定されたGitHubリポジトリをローカルにクローンし、コードをGrepします。パブリックおよびプライベートリポジトリの両方をサポートします。

#### 入力パラメータ

```rust
pub struct GrepRequest {
    // リポジトリURL（必須）- 以下の形式をサポート
    // - https://github.com/{user_name}/{repo}
    // - git@github.com:{user_name}/{repo}.git
    // - github:{user_name}/{repo}
    pub repository: String,
    // ブランチまたはタグ（オプション、デフォルトはmainまたはmaster）
    pub ref_name: Option<String>,
    // 検索パターン（必須）
    pub pattern: String,
    // 大文字小文字を区別するかどうか（オプション、デフォルトはfalse）
    pub case_sensitive: Option<bool>,
    // 正規表現を使用するかどうか（オプション、デフォルトはtrue）
    pub use_regex: Option<bool>,
    // 検索するファイルの拡張子（オプション、例: ["rs", "toml"]）
    pub file_extensions: Option<Vec<String>>,
    // 検索から除外するディレクトリ（オプション、例: ["target", "node_modules"]）
    pub exclude_dirs: Option<Vec<String>>,
}
```

#### 実装詳細

1. リポジトリURLを解析し、ユーザー名とリポジトリ名を抽出
2. 一時ディレクトリを生成: `{system_temp_dir}/mcp_https__github_com__{user_name}__{repo}_{hash}`
   - ここで `hash` は `hash(user_name + repo + process_seed.to_string())` で生成
3. リポジトリが既にクローン済みかチェック
   - クローン済み:
     - `{system_temp_dir}/mcp_https__github_com__{user_name}__{repo}_{hash}` ディレクトリが存在する場合
     - `git fetch origin` を実行
     - 指定されたブランチが存在しない場合は、デフォルトで `master` または `main` ブランチを使用
     - `git checkout <branch_or_tag>` を実行
     - `git pull origin <branch>` を実行（タグの場合はこの操作をスキップ）
   - 未クローン:
     - gitoxide crateを使用して浅いクローンを実行 (`--depth=1`)
     - 指定されたブランチ/タグをチェックアウト
4. lumin crateを使用してコード検索を実行
5. 結果を標準レスポンス形式で返す

#### 一時ディレクトリ管理

- 既存のディレクトリが存在する場合は再利用
- ディレクトリは以下の場合に更新:
  - 最後の更新から24時間以上経過
  - 要求されたブランチ/タグが現在のものと異なる
- MCPシャットダウン時に自動クリーンアップ
- 7日以上アクセスされていないディレクトリは自動削除
- 総容量制限（デフォルト: 10GB）に達した場合、最も古いリポジトリから削除

#### 戻り値

```rust
pub struct GrepResult {
    // 一致したファイルのリスト
    pub matches: Vec<FileMatch>,
    // 検索に関する統計情報
    pub stats: SearchStats,
}

pub struct FileMatch {
    // ファイルのパス（リポジトリルートからの相対パス）
    pub path: String,
    // 一致した行とその内容
    pub line_matches: Vec<LineMatch>,
}

pub struct LineMatch {
    // 行番号
    pub line_number: u32,
    // 行の内容
    pub line: String,
    // 行内の一致した範囲（開始位置と長さ）
    pub ranges: Vec<(usize, usize)>,
}

pub struct SearchStats {
    // 検索されたファイルの総数
    pub files_searched: u32,
    // 見つかった一致の総数
    pub total_matches: u32,
    // 少なくとも1つの一致があったファイルの数
    pub files_with_matches: u32,
    // 検索にかかった時間（ミリ秒）
    pub execution_time_ms: u64,
}
```

#### レスポンス例

```json
{
  "success": true,
  "data": {
    "matches": [
      {
        "path": "src/main.rs",
        "line_matches": [
          {
            "line_number": 42,
            "line": "    async fn process_request(&self, req: Request) -> Result<Response> {",
            "ranges": [[4, 10]]
          }
        ]
      }
    ],
    "stats": {
      "files_searched": 156,
      "total_matches": 23,
      "files_with_matches": 5,
      "execution_time_ms": 345
    }
  },
  "metadata": {
    "execution_time_ms": 1234
  }
}
```

### 3. GitHub リポジトリのブランチ/タグ一覧ツール

指定されたGitHubリポジトリのブランチとタグの一覧を取得します。

#### 入力パラメータ

```rust
pub struct ListRefsRequest {
    // リポジトリURL（必須）- 以下の形式をサポート
    // - https://github.com/{user_name}/{repo}
    // - git@github.com:{user_name}/{repo}.git
    // - github:{user_name}/{repo}
    pub repository: String,
}
```

#### 実装詳細

- Grepツールと同じ方法でリポジトリのローカルチェックアウトを作成または再利用
- gitoxide crateを使用してブランチとタグ情報を抽出

#### 戻り値

```rust
pub struct RefsResult {
    // ブランチの一覧
    pub branches: Vec<String>,
    // タグの一覧
    pub tags: Vec<String>,
}
```

#### レスポンス例

```json
{
  "success": true,
  "data": {
    "branches": ["main", "develop"],
    "tags": ["v0.0.1", "v0.1.0"]
  },
  "metadata": {
    "execution_time_ms": 123
  }
}
```

## エラー処理

MCPは以下のエラー状況を適切に処理します：

### API関連エラー

- ネットワークエラー: 自動再試行（指数バックオフ）
- 認証エラー: トークン検証を試み、ユーザーに通知
- レート制限エラー: 待機時間を計算し、次の可能なリクエスト時間を返す

### Git操作エラー

- クローン失敗: 詳細なエラーメッセージとリポジトリ情報の検証
- チェックアウト失敗: 存在しないブランチ/タグについての情報提供
- 権限エラー: アクセス権の問題を明確に説明

### 一時ファイルエラー

- ディスク容量不足: クリーンアップを試み、必要なスペースを通知
- 書き込み権限エラー: 代替ディレクトリを試行

すべてのエラーは標準的なRust Result型を通じて処理され、意味のあるエラーメッセージとともに返されます。

### エラーレスポンス例

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "rate_limit_exceeded",
    "message": "GitHub API rate limit exceeded. Reset at 2023-05-01T12:00:00Z",
    "details": {
      "rate_limit_reset": 1620000000,
      "rate_limit_remaining": 0
    }
  },
  "metadata": {
    "execution_time_ms": 45
  }
}
```

## 入力検証

すべての入力は使用前に検証され、無効な入力は早期にエラーとして報告されます：

### リポジトリURL検証

- 形式チェック: 有効なGitHubリポジトリURLであることを確認
- 存在チェック: リポジトリが存在することを確認（オプション）

### 検索クエリ検証

- 空文字列は許可されない
- 安全でない文字またはパターンはエスケープまたは拒否

### ブランチ/タグ検証

- 存在するブランチ/タグであることを確認
- 無効な文字を含まないことを確認

### Grep検索パターン検証

- 正規表現として有効であることを確認（use_regex=trueの場合）
- パターンの複雑さをチェック（過度に複雑なパターンはパフォーマンスに影響する可能性がある）

## インターフェース

MCPは以下のインターフェースを通じてアクセスできます：

### Rustライブラリとして

```rust
// ライブラリを初期化
let mcp = ModelContextProtocol::new(Config::default());

// GitHubリポジトリを検索
let results = mcp.search_repositories("rust http client").await?;

// リポジトリコードをグレップ
let grep_results = mcp.grep_repository(
    "https://github.com/simonw/llm",
    Some("main"),
    "async fn",
    Default::default()
).await?;

// ブランチとタグを取得
let refs = mcp.list_repository_refs("github:simonw/llm").await?;
```

### コマンドラインツールとして

```bash
# リポジトリを検索
mcp search "rust http client"

# コードをグレップ
mcp grep github:simonw/llm "async fn" --branch=main

# ブランチとタグを表示
mcp refs github:simonw/llm
```

## パフォーマンス考慮事項

MCPは以下の戦略を通じて高性能を維持します：

### リポジトリ操作の最適化

- 完全なクローンではなく浅いクローンを使用（--depth=1）
- スパースチェックアウトのサポート（特定のディレクトリのみ）
- 既存クローンの再利用と効率的な更新（git pull）

### 検索最適化

- インデックスベースの検索（可能な場合）
- 並列検索の実行
- 大規模リポジトリでのストリーミング結果

### メモリ使用量

- 大きなリポジトリの検索時のメモリ消費を制限
- 結果のバッファリングとページング

## セキュリティ考慮事項

MCPは以下のセキュリティ対策を実装しています：

### 認証情報の保護

- GitHub APIトークンは安全に保存・処理
- トークンはログに記録されない
- 環境変数または安全なストレージからのみ読み取り

### コード実行の防止

- ダウンロードしたコードは解析のみで実行しない
- スクリプトや実行可能ファイルの実行は防止

### サンドボックス化

- すべての操作は一時ディレクトリに限定
- 親ディレクトリへのアクセスは防止

### 入力サニタイズ

- すべてのユーザー入力は使用前に検証・サニタイズ
- コマンドインジェクション攻撃を防止

## 実装要件

### 依存クレート

- local fileのgrepには`lumin`クレートを使用
- 一時ディレクトリの管理には`tempfile`クレートを使用
- gitのcheckoutには`gitoxide`クレートを使用

### 参考実装

- `gitcodes-mcp/rust-sdk`以下のソースを参考にする
- MCPのツールのレスポンスは文字列ではなく`CallToolResult`型を使用

### 同時実行の管理

- 同じリポジトリへの複数のリクエストは一時ディレクトリを共有
- 共有リソースへのアクセスはRustの標準的な同期メカニズムで保護
