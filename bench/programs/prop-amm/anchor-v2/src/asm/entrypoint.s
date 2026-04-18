.globl entrypoint
entrypoint:
    ldxb r3, [r2+0]
    jne r3, 0, dispatch
    ldxdw r3, [r1+0]
    jlt r3, 2, err_few_accounts
    ldxdw r3, [r1+88]
    jne r3, 48, err_bad_data_len
    ldxb r3, [r1+10]
    jeq r3, 0, err_not_writable
    ldxb r3, [r1+10393]
    jeq r3, 0, err_not_signer
    ldxdw r3, [r1+10400]
    lddw r4, 0x0A529CE2636C4AEA
    jne r3, r4, err_wrong_auth
    ldxdw r3, [r1+10408]
    lddw r4, 0xF9C52E137B50F5BE
    jne r3, r4, err_wrong_auth
    ldxdw r3, [r1+10416]
    lddw r4, 0x927BBEBEAE764795
    jne r3, r4, err_wrong_auth
    ldxdw r3, [r1+10424]
    lddw r4, 0x2CD2461469EA1E42
    jne r3, r4, err_wrong_auth
    // Unaligned 8B load: ix_data is aligned 1; sBPFv1+ allows this.
    ldxdw r3, [r2+1]
    stxdw [r1+136], r3
    mov64 r0, 0
    exit
err_few_accounts:
    mov64 r0, 101
    exit
err_bad_data_len:
    mov64 r0, 102
    exit
err_not_writable:
    mov64 r0, 103
    exit
err_not_signer:
    mov64 r0, 104
    exit
err_wrong_auth:
    mov64 r0, 105
    exit
dispatch:
    call __anchor_dispatch
    exit
