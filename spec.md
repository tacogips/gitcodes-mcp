# Library Specification

現在のこの仕様は断片的で将来的に清書される予定です。

## model context protocolとして下記の機能を提供します

mcpはstartしたときにランダムなseed値をオンメモリに保持します。このseed値は並行してこのMCPが実行された際にlocal directory pathがコンフリクトを避けるために使用されます。
このseedを`process seed`と呼びます　

### 1. github repository search tool

`https://api.github.com/search/repositories?q={query}` urlでrepository searchを行う
reference docs: https://docs.github.com/en/rest/search/search

- response format(example)

```json
[
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
    "has_issues": true,
    "has_projects": true,
    "has_downloads": true,
    "has_wiki": false,
    "has_pages": false,
    "has_discussions": true,
    "archived": false,
    "disabled": false,
    "license": {
      "key": "apache-2.0",
      "name": "Apache License 2.0",
      "spdx_id": "Apache-2.0",
      "url": "https://api.github.com/licenses/apache-2.0"
    },
    "allow_forking": true,
    "is_template": false,
    "topics": ["ai", "llms", "openai"],
    "visibility": "public",
    "open_issues": 374,
    "watchers": 7294,
    "default_branch": "main"
  }
]
```

### 2. github の特定のrepositoryのコードをgrepする tool

下記のいずれかを受取り、codeをlocal direcotryにcheckoutする。 checkout 方法は
下記`how to implementation`を参照

- https://github.com/{user_name}/{repo}
- git@github.com:{user_name}/{repo}.git
- github:{user_name}/{repo}

その後任意に{branch}または{tag}を受取checkout。
その後luminを使用してコードをgrepする。luminの使用方法はcrate.io参照

##### how to implementation

与えられたurlから、`https__github_com__{user_name}__{repo}_{hash}`形式のディレクトリを作製する。
このhashはuser_name,repo,process seedから算出されます。
ディレクトリはsystemのtemp directoryに作製する。

- response example

```json
[
  {
    "name": "file_path",
    "type": "PathBuf",
    "visibility": "public"
  }
]
```

### 3. github の特定のrepositoryのbranch,tag一覧を取得する tool

`2. github の特定のrepositoryのコードをgrepする tool`と同じ方法でlocalにcheckoutする。
checkoutしたrepositoryのbranchとtag一覧を取得します

取得にはgitoxide crateを使用　

- response example

```json
  {
    "branchs":[
      "main",
      "develop"
    ],

    "tags":[
      "v0.0.1",
      "develop",
    ]

  }
]
```

## implementation rule

local fileのgrepにはlumin crateを使用
tempdirectoryの管理には`tempfile` crateを使用
gitのcheckoutにはgitoxide crateを使用　

mcpの実装は　
gitcodes-mcp/rust-sdk 以下のソースを参考にしてください。
特にmcpのtoolのresponseを文字列ではなく CallToolResult型にしたいので参考に
