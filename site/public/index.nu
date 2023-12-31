print -e "GET:" $env.GET
print -e "POST:" $env.POST
print -e "SESSION:" $env.SESSION
print -e "FILES:" $env.FILES


index "Mikayla"

$env.SESSION.name = $"Mikayla (date now)"
let formatted_date = (date now | into string | str replace " " "-" --all)
$env.RES_HEADERS."Set-Cookie" = $"nu-cookie-nu-headr=($formatted_date)"

def index [name] {
    print ("<html>
              <head>
                <script src='https://unpkg.com/htmx.org@1.9.5'></script>
              </head>
              <body>")

    if ('name' in $env.GET) {
        print ($"<h1>Hello, ($env.GET.name)</h1>")
    } else {
        print ($"<h1>Hello, ($name)</h1>")
    }

    print ($"<h2>Files: </h2> <pre>")
    for $it in ($env.FILES | transpose key value) {
        print -n ($"($it.key) - ($it.value):")
        open $it.value | print
    }
    print ($"</pre>")


    print ($"<h2>SESSION: </h2> <pre>")
    for $it in ($env.SESSION | transpose key value) {
        print ($"($it.key): ($it.value)")
    }
    print ($"</pre>")

    print ($"<h2>Headers: </h2> <pre>")
    for $it in ($env.REQ_HEADERS | transpose key value) {
        print ($"($it.key): ($it.value)")
    }
    print ($"</pre>")

    print "<h2>Comments:</h2>"

    # hx-post='/comments' hx-target='.comments' hx-swap='beforebegin' hx-on::after-request='this.reset\(\)
    print ($"
        <form enctype='multipart/form-data' action='/' method='post'>
        Make a new comment: <br/>
        <input name='comment' value='' placeholder='comment'>
        <input name='username' value='' placeholder='username'>
        <input name='file' value='' type='file' placeholder='username'>
        <input type='submit' value='Submit'>
    </form>
    ")

    print "<div class='comments'>"
    source comments.nu
    print "</div>"

    print "</body></html>"
}
