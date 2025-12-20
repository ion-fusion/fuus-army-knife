FuusArmyKnife
=============

FuusArmyKnife is a Fusion multi-tool written in Rust. Today, it provides:

- Auto-formatting
- Enforcement of code style (similar to Java Checkstyle)
- `check-correctness-watch` sub-command that watches your changes as you write Fusion code and reports on errors that it detects

FuusArmyKnife uses its own parser and implementation of the Fusion Grammar. It is sufficient for
simple formatting tasks but does not have a full understanding of the resolved binding index
post-macro expansion. The default formatting rules have hard-coded references to certain symbol
names but does not presently have a mechanism to stay in sync with the Fusion library API at this
time.

Long-term, the intention is that the Fusion distribution will vend official tooling both in CLI and
in IDE plugin/language server form that will supplant the functionality currently provided by this
binary. In the short-term absence of those tools, this can fill the gap with the caveats above
in mind.

Why is it called "Fuus", and how do I pronounce it?
---------------------------------------------------

The name was intended to sound somewhat similar to "Swiss Army Knife", but for Fusion. "Fuus" is
pronounced like "food" with a "S" instead of a "D". You can also call it `fuusak` since that's what
the compiled CLI binary's name is.
