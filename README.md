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

- You can test the nu page scripts by running them directly with the `$env.GET` and `$env.POST` variables set, no complex mocking required!
- Since nu is table based, you can mostly just use normal nu code to interface with the database.

TODO:
- [x] Add cookies for identifying sessions
- [x] Optimize concurrency for session and session identifier
- [x] Implement session and header parsing via nuphp.nu script
- [x] Figure out a way to do file uploads
- [ ] Add a testing harness
- [ ] Add a $COOKIE output variable
- [ ] Add defunctionalized closure passing by serialzing the closure's enviroment to a file
- [ ] file bug about highlighting for $env.REQUEST_PATH
- [ ] File highlight bug about # with nothing following it
