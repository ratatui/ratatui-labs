Output target/betamax/renderers/split.gif
Output target/betamax/renderers/split.png
Output target/betamax/renderers/split.json

Require cargo

Set Shell "bash"
Set Width 1200
Set Height 650
Set WindowBar Colorful
Set BorderRadius 8
Set TypingSpeed 35ms
Set CursorBlink false
Set WaitTimeout 30s

Hide
Type "cd \"$REPO_DIR\""
Enter
Wait
Type "cargo run --quiet -p ratatui-command-palette --example command-palette -- --renderer split"
Enter
Wait+Screen "renderer: split"
Show
Wait+Screen "Details"
Screenshot target/betamax/renderers/split-open.png
State target/betamax/renderers/split-open.json
Sleep 2s

Down
Sleep 500ms
Wait+Screen "Expand row"
Screenshot target/betamax/renderers/split-details.png
Sleep 2s

Escape
Wait
