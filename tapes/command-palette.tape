Output target/betamax/command-palette.gif
Output target/betamax/command-palette.png
Output target/betamax/command-palette.json

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
Type "cargo build --quiet -p ratatui-command-palette --example command-palette && printf 'built-command-palette\n'"
Enter
Wait+Screen "built-command-palette"
Show

Type "cargo run --quiet -p ratatui-command-palette --example command-palette"
Enter
Wait+Screen "Command Palette"
Screenshot target/betamax/command-palette-open.png
State target/betamax/command-palette-open.json
Sleep 2s

End
Sleep 500ms
Wait+Screen "Quit"
Screenshot target/betamax/command-palette-scrolled.png
Sleep 2s

Type "theme"
Sleep 500ms
Wait+Screen "Switch theme"
Screenshot target/betamax/command-palette-filtered.png
Sleep 2s

Down
Sleep 500ms
Enter
Wait+Screen "Catppuccin"
Screenshot target/betamax/command-palette-input.png
Sleep 2s

Down
Sleep 500ms
Wait+Screen "preview theme.switch (theme=github-dark)"
Screenshot target/betamax/command-palette-preview.png
Sleep 2s

Escape
Wait+Screen "cancelled"
Screenshot target/betamax/command-palette-cancelled.png
Sleep 2s

Type "renderer"
Sleep 500ms
Wait+Screen "Switch renderer"
Enter
Wait+Screen "Modal"
Down
Sleep 500ms
Down
Sleep 500ms
Wait+Screen "preview renderer.switch (renderer=split)"
Enter
Wait+Screen "invoked renderer.switch (renderer=split)"
Screenshot target/betamax/command-palette-renderer-split.png
Sleep 2s

Type "search"
Sleep 500ms
Wait+Screen "Open search"
Enter
Wait+Screen "Query:"
Screenshot target/betamax/command-palette-text-input.png
Sleep 2s

Type "widgets"
Sleep 500ms
Wait+Screen "input: widgets"
Screenshot target/betamax/command-palette-text-typed.png
Sleep 2s

Enter
Wait+Screen "invoked search.open (query=widgets)"
Screenshot target/betamax/command-palette-search-invoked.png
Sleep 2s

Type "debug"
Sleep 500ms
Wait+Screen "Toggle debug mode"
Enter
Wait+Screen "Yes"
Screenshot target/betamax/command-palette-bool-input.png
Sleep 2s

Down
Sleep 500ms
Wait+Screen "preview debug.toggle (enabled=false)"
Screenshot target/betamax/command-palette-bool-preview.png
Sleep 2s

Enter
Wait+Screen "invoked debug.toggle (enabled=false)"
Screenshot target/betamax/command-palette-bool-invoked.png
Sleep 2s

Type "open"
Sleep 500ms
Wait+Screen "Open document"
Enter
Wait+Screen "invoked document.open"
Screenshot target/betamax/command-palette-invoked.png
State target/betamax/command-palette-invoked.json
Sleep 5s

Type "quit"
Sleep 500ms
Wait+Screen "Quit"
Down
Sleep 500ms
Wait+Screen "Quit"
Screenshot target/betamax/command-palette-quit.png
Sleep 2s

Enter
Wait
