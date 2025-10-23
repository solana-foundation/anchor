# `missing_account_reload`

### What it does
Identifies access of an account without calling `reload()` after a CPI.

### Why is this bad?
After a CPI, deserialized accounts do not have their data updated automatically.
Accessing them without calling `reload` may lead to stale data being loaded.
