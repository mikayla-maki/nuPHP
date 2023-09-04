use ../nu-php.nu *

print -e "GET:" $env.GET
print -e "POST:" $env.POST

index "Mikayla"

def index [name] {
    source "../templates/header.nu"

    if ('name' in $env.GET) {
        print ($"<h1>Hello, ($env.GET.name)</h1>")
    } else {
        print ($"<h1>Hello, ($name)</h1>")
    }

    print ($"
    <form method='POST' action=''>
        <input name='hello' value='test'>
        <input type='submit'>
    </form>
    ")

    source "../templates/footer.nu"
}
