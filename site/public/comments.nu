if ('comment' in $env.POST and 'username' in $env.POST) {
    open site/db/main.db |
    query db $"INSERT INTO comments \(comment, username\) VALUES \('($env.POST.comment)', '($env.POST.username)'\) RETURNING *" |
    select id comment username |
    each {|it|
      comment $it.id $it.comment $it.username
    }
} else {
    if ('comment_id' in $env.GET) {
        open site/db/main.db |
         get "comments" |
         select id comment username |
         where id == ($env.GET.comment_id | into int) |
         each {|it|
            comment $it.id $it.comment $it.username
         }
    } else {
        open site/db/main.db |
         get "comments" |
         sort-by comment |
         select id comment username |
         each {|it|
            comment $it.id $it.comment $it.username
         }
    }
}

def comment [id, comment, username] {
    print $"<div class='($id)'>($comment) <span>- ($username)</span></div>"
}
