#!/opt/homebrew/bin/nu
 
let cudo_count = (rg --no-line-number "File: (.*yaml)" cudo_manpage_output.log -r "$1" | wc -l | into int | $in + 13)
print $"Cudo count ($cudo_count)"
let rocinante_count = (rg --no-line-number "File: (.*yaml)" rocinante_manpage_output.log -r "$1" | wc -l | into int)
print $"Rocinante count ($rocinante_count)"
let google_count = (rg --no-line-number "File: (.*yaml)" google_manpage_output.log -r "$1" | wc -l | into int)
print $"Google count ($google_count)"

let google_total = (ls inputs/manpages | wc -l | into int)
let cudo_total = (ls cudo-in-progress | wc -l | into int)
let rocinante_total = (ls rocinante-in-progress | wc -l | into int)

let total = ($google_total + $cudo_total + $rocinante_total)
let count = ($google_count + $cudo_count + $rocinante_count)

print $"Finished ($count)/($total)"
