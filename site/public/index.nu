use ../nu-php.nu *

print -e "GET:" $env.GET
print -e "POST:" $env.POST

index "Mikayla"

def index [name] {
    print "<html><body>"

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

    print ($"
    <form method='POST' action=''>
        Comment:
        <input name='comment' value='test'>
        <input name='author' value=''>
        <input type='submit'>
    </form>
    ")

    print "</body></html>"
}
