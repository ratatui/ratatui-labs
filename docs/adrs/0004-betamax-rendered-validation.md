# 0004 Betamax Rendered Validation

## Status

Accepted.

## Context

The command palette is a terminal UI feature. Unit tests can prove filtering, selection, invocation,
and state transitions, but they do not prove the actual terminal rendering.

The visible behavior depends on:

- layout and spacing
- colors and selection highlighting
- title and status bars
- wrapping and truncation
- keyboard interaction timing
- alternate-screen capture
- preview and invocation status text

A PTY smoke test can prove that a command starts and exits, but it cannot provide good evidence for
the visual behavior reviewers care about.

## Decision

Use Betamax as the rendered validation path for command palette UI changes. The default recipe runs
`tapes/command-palette.tape` and writes PNG, GIF, and state artifacts under `target/betamax/`.

The tape should drive the real example application and capture checkpoint artifacts for important
states, including:

- initial palette
- scrolled results
- filtered results
- choice input
- text input
- boolean input
- preview selection
- preview cancellation
- final invocation

The tape should be paced for human review. Use short input pauses, longer stable-screen holds, and
checkpoint PNG/state artifacts instead of relying only on a fast animated GIF.

## Consequences

Reviewers get concrete visual evidence for the terminal UI, and regressions in layout or highlight
behavior are easier to spot. The same tape also exercises the real example rather than a synthetic
renderer-only fixture.

The cost is slower validation and more moving parts. Betamax is still experimental, so failures can
come from the tool, terminal capture, media generation, or the app. When that friction exposes a
tool gap, record a concrete Betamax improvement idea rather than treating it as incidental local
noise.

## Alternatives Considered

### Unit Tests Only

Unit tests are still required for state and action behavior. They are insufficient for colors,
spacing, wrapping, row highlighting, and interaction timing.

### PTY Smoke Test

A PTY smoke test is useful for lifecycle checks. It is not enough to prove user-visible terminal
rendering.

### Manual Screenshots Only

Manual screenshots can be useful during development, but they are easy to forget and hard to
reproduce. Betamax gives the repo a repeatable rendered validation path.

## Revisit When

- Betamax tape authoring friction blocks routine iteration
- artifact naming or inspection becomes hard for reviewers
- the example gains expansion, additional wrapping behavior, or another input mode
- a CI path needs a lighter rendered-validation mode
