Output target/betamax/renderers/inline.gif
Output target/betamax/renderers/inline.png
Output target/betamax/renderers/inline.json

Require cargo

Set Shell "bash"
Set Theme "GitHub Dark"
Set Width 1200
Set Height 620
Set WindowBar Colorful
Set BorderRadius 8
Set TypingSpeed 35ms
Set CursorBlink false
Set WaitTimeout 30s

Hide
Type "cd /Users/joshka/local/ratatui-labs/default"
Enter
Wait
Type "cargo run --quiet -p ratatui-command-palette --example command-palette -- --renderer inline"
Enter
Wait+Screen "renderer: inline"
Show
Screenshot target/betamax/renderers/inline-open.png
State target/betamax/renderers/inline-open.json
Sleep 2s

Type "debug"
Sleep 500ms
Wait+Screen "Toggle debug mode"
Enter
Wait+Screen "Yes"
Down
Sleep 500ms
Wait+Screen "preview debug.toggle (enabled=false)"
Screenshot target/betamax/renderers/inline-bool-preview.png
Sleep 2s

Enter
Wait+Screen "invoked debug.toggle (enabled=false)"
Escape
Wait
