# Rustalyzer

Basic static analysis for Rust programs.

Currently Rustalyzer shows the ratio of unsafe statements to all statements. For example:

```
rustalyzer a.rs b.rs c.rs
a.rs: 12/15
b.rs: 0/4
c.rs: 5/19
total: 17/38
```
