Output target/betamax/renderers/modal.gif
Output target/betamax/renderers/modal.png
Output target/betamax/renderers/modal.json

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
Type "cargo run --quiet -p ratatui-command-palette --example command-palette -- --renderer modal"
Enter
Wait+Screen "renderer: modal"
Show
Screenshot target/betamax/renderers/modal-open.png
State target/betamax/renderers/modal-open.json
Sleep 2s

Type "theme"
Sleep 500ms
Wait+Screen "Switch theme"
Screenshot target/betamax/renderers/modal-filtered.png
Sleep 2s

Escape
Wait
