# Library Specification

現在のこの仕様は断片的で将来的に清書される予定です。

## model context protocolとして下記の機能を提供します

### github のコードをgrep

https://github.com/{user_name}/{repo}
git@github.com:{user_name}/{repo}.git

### github のレポジトリのfile名検索

### repository 検索

- github repository search
  `https://api.github.com/search/repositories?q={query}` urlでrepository searchを行う
  doc: https://docs.github.com/en/rest/search/search

- git lab repospository search

  doc: https://docs.gitlab.com/api/search/

# hint to implementation

local fileのgrep,file searchには lumin crateを使用
gitのcheckoutにはgitoxide crateを使用　

gitcodes-mcp/rust-sdk 以下のソースを参考にしてください
