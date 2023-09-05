cd $env.FILE_PWD
rm main.db
sqlite3 main.db "VACUUM;"

open main.db | query db ("
    CREATE TABLE comments(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        comment TEXT,
        username TEXT
    );
    -- Add more migrations here
")

print "Created DB main.db"
