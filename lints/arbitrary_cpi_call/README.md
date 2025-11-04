# `arbitraty_cpi_call`

### What it does
Identifies CPI calls made using user-controlled program IDs without validations.

### Why is this bad?
Unvalidated program IDs in CPI calls let users to trigger arbitrary programs, leading to potential security breaches or fund loss.

