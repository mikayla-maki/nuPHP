nuPHP is a small Rust based web server for responding to HTTP requests with the Nushell language. You must have `nu` available in your path to run this webserver.

Use `nu site/db/init.nu` to create the database.

Use `cargo run` to start the webserver

See the `site` directory for the current example for how you could use this server to create a website.

Some notes on how it all fits together:

- The contents of the `site/public` directory will be available (via the web server) at `/`
- Non-Nushell files must have a file extension (not `.nu`) and will be served as static files.
- Nu files will be executed and the standard output will be sent as the response. Standard error will be printed to the console.
- Requests for paths without an extension will have `.nu` added automatically.
- All nu scripts are executed with `$env.GET`, `$env.POST` and `$env.HEADERS` available as variables.

Some cool observations:

- You can easily create sub paths by creating a file with a name (`users.nu`) and the subpath in a folder with the same name (`users/details.nu`).
- You can test the nu page scripts by running them directly with the `$env.GET` and `$env.POST` variables set, no complex mocking required!
- Since nu is table based, you can mostly just use normal nu code to interface with the database.
