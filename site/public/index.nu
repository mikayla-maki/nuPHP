print -e "GET:" $env.GET
print -e "POST:" $env.POST

index "Mikayla"

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

    print ($"<h2>Headers: </h2> <pre>")
    for $it in ($env.HEADERS | transpose key value) {
        print ($"($it.key): ($it.value)")
    }
    print ($"</pre>")

    print "<h2>Comments:</h2>"

    print ($"
    <form hx-post='/comments' hx-target='.comments' hx-swap='beforebegin' hx-on::after-request='this.reset\(\)'>
        Make a new comment: <br/>
        <input name='comment' value='' placeholder='comment'>
        <input name='username' value='' placeholder='username'>
        <input type='submit' value='Submit'>
    </form>
    ")

    print "<div class='comments'>"
    source comments.nu
    print "</div>"

    print "</body></html>"
}
