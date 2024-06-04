#!/bin/bash
diesel setup
diesel migration generate auto_anime
for dir in migrations/*_auto_anime; do
    if [ -d "$dir" ]; then
        cp init.sql "$dir/up.sql"
    fi
done
diesel migration run
