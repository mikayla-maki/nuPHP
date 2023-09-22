# TODO: After sourcing the file, get the current enviroment
# variables, and output them to the web server
# Use export_env to get enviroment data out of nu:
# export-env {RES_HEADER: .. SESSION: ..}
# And in this script, capture the stdout of $PATH and
# the enviroment headers and send them in a way the webserver
# can understand
# Probably something simple like: "Body:\n{}\nHeaders:\n{format_this}\nSession:\n{format_this}"

source $PATH
