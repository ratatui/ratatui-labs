Output target/betamax/renderers/flat.gif
Output target/betamax/renderers/flat.png
Output target/betamax/renderers/flat.json

Require cargo

Set Shell "bash"
Set Width 1200
Set Height 620
Set WindowBar Colorful
Set BorderRadius 8
Set TypingSpeed 35ms
Set CursorBlink false
Set WaitTimeout 30s

Hide
Type "cd \"$REPO_DIR\""
Enter
Wait
Type "cargo run --quiet -p ratatui-command-palette --example command-palette -- --renderer flat"
Enter
Wait+Screen "renderer: flat"
Show
Screenshot target/betamax/renderers/flat-open.png
State target/betamax/renderers/flat-open.json
Sleep 2s

Type "theme"
Sleep 500ms
Wait+Screen "Switch theme"
Enter
Wait+Screen "Catppuccin"
Down
Sleep 500ms
Wait+Screen "preview theme.switch (theme=github-dark)"
Screenshot target/betamax/renderers/flat-choice-preview.png
Sleep 2s

Escape
Wait+Screen "cancelled"
Escape
Wait
