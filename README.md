# autoAnimeR
autoAnime in Rust 🦀️

## db
```
brew install sqlite

cd app
diesel setup
diesel migration generate auto_anime
vim migrations/*/up.sql
diesel migration run
```