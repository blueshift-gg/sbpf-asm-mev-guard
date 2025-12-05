// Global offsets
.equ NUM_ACCOUNTS, 0x0000
.equ MEV_GUARD_HEADER, 0x0008
.equ MEV_GUARD_DATA_LEN, 0x0058
.equ MEV_GUARD_LAST_UPDATED_SLOT, 0x0060 // u64 (8 bytes)
.equ MEV_GUARD_BITMAP, 0x0068 // [u8;N][13500] (13500 bytes)

// Expected values
.equ SLOTS_PER_EPOCH, 0x000069780
.equ SLOTS_PER_ROTATION, 0x0004
.equ EXPECTED_MEV_GUARD_DATA_LEN, 0x34C4
.equ SIGNER_NODUP, 0x01ff
.equ EXPECTED_ADMIN_KEY_0, 0x3713371337133713
.equ EXPECTED_ADMIN_KEY_1, 0x3713371337133713
.equ EXPECTED_ADMIN_KEY_2, 0x3713371337133713
.equ EXPECTED_ADMIN_KEY_3, 0x3713371337133713

// Update-specific offsets
.equ ADMIN_HEADER, 0x5d30
.equ ADMIN_KEY_0, 0x5d38
.equ ADMIN_KEY_1, 0x5d40
.equ ADMIN_KEY_2, 0x5d48
.equ ADMIN_KEY_3, 0x5d50

// Instruction data
.equ INSTRUCTION_DATA_LEN, 0x8590
.equ INSTRUCTION_LAST_UPDATE_SLOT, 0x0008 // u64 (8 bytes) 0x8598
.equ INSTRUCTION_OFFSET, 0x0010 // u16 (2 bytes) 0x85a0
.equ INSTRUCTION_UPDATE_DATA, 0x0012 // Variable length

// Clock Syscall offsets
.equ CLOCK_SYSCALL_SLOT, 0x0000 // u64 (8 bytes)
.equ CLOCK_SYSCALL_EPOCH_START_TIMESTAMP, 0x0008 // i64 (8 bytes)
.equ CLOCK_SYSCALL_EPOCH, 0x0010 // u64 (8 bytes)
.equ CLOCK_SYSCALL_LEADER_SCHEDULE_EPOCH, 0x0018 // u64 (8 bytes)
.equ CLOCK_SYSCALL_UNIX_TIMESTAMP, 0x0020 // i64 (8 bytes)

.globl entrypoint
entrypoint:
  // If we have 2 accounts, jump to update_mev_guard ix
  ldxdw r9, [r1+NUM_ACCOUNTS]
  jeq r9, 2, update_mev_guard
  // If we don't have exactly 1 account, throw INVALID_ACCOUNT_DATA error
  jne r9, 1, account_error
  // Store offset to MEV guard bitmap in r6 so we can overwrite r1 with a syscall
  mov64 r6, r1
  add64 r6, MEV_GUARD_BITMAP
  // Get clock account from syscall
  mov64 r1, r10
  add64 r1, -0x0040
  call sol_get_clock_sysvar
  // Store current slot
  ldxdw r9, [r1+CLOCK_SYSCALL_SLOT]
  // Load most up to date slot from MEV guard
  ldxdw r8, [r6-0x0008]
  // Check if Bitmap is stale (highest updated slot < current slot)
  jlt r8, r9, stale_mev_guard
  // Get index of current leader rotation
  mod64 r9, SLOTS_PER_EPOCH // Modulo current slot by number of slots in an epoch
  div64 r9, SLOTS_PER_ROTATION // Divide by number of slots per leader rotation (4)
  // Calculate bitmask from rotation index
  mov64 r8, r9
  mod64 r8, 0x0008
  lddw r7, 0x0001
  lsh64 r7, r8
  // Get byte offset of current rotation
  div64 r9, 0x0008 // Divide by 8 to get byte offset
  // Check bitwise and of current rotation byte
  add64 r6, r9 // Add offset of slot index
  ldxb r6, [r6+0x0000]
  jset r6, r7, blocklisted // If bitmask ≠ 0, error
  ja success
blocklisted:
  lddw r1, e_blocklisted
  mov32 r2, 30
  call sol_log_
  lddw r0, 0x0400000000 // INVALID_ACCOUNT_DATA
  exit
account_error:
  lddw r1, e_account_error
  mov32 r2, 26
  call sol_log_
  lddw r0, 0x0400000000 // INVALID_ACCOUNT_DATA
  exit
admin_error:
  lddw r1, e_admin_error
  mov32 r2, 21
  call sol_log_
  lddw r0, 0x1a00000000 // INVALID_ACCOUNT_DATA
  exit
stale_mev_guard:
  lddw r1, e_stale_mev_guard
  mov32 r2, 20
  call sol_log_
  lddw r0, 0x0400000000 // INVALID_ACCOUNT_DATA
  exit
update_mev_guard:
  // Enforce the length of the MEV guard account
  ldxdw r9, [r1+MEV_GUARD_DATA_LEN]
  jne r9, EXPECTED_MEV_GUARD_DATA_LEN, account_error
  // Signer check
  ldxh r9, [r1+ADMIN_HEADER]
  jne r9, SIGNER_NODUP, admin_error
  // Check for a blank admin authority
  ldxdw r9, [r1+ADMIN_KEY_0]
  lddw r8, EXPECTED_ADMIN_KEY_0
  jne r8, r9, admin_error
  ldxdw r9, [r1+ADMIN_KEY_1]
  lddw r8, EXPECTED_ADMIN_KEY_1
  jne r8, r9, admin_error
  ldxdw r9, [r1+ADMIN_KEY_2]
  lddw r8, EXPECTED_ADMIN_KEY_2
  jne r8, r9, admin_error
  ldxdw r9, [r1+ADMIN_KEY_3]
  lddw r8, EXPECTED_ADMIN_KEY_3
  jne r8, r9, admin_error
  // Load and store last updated slot
  mov64 r7, r1
  add64 r7, INSTRUCTION_DATA_LEN
  ldxdw r6, [r7+INSTRUCTION_LAST_UPDATE_SLOT]
  stxdw [r1+MEV_GUARD_LAST_UPDATED_SLOT], r6
  // Store offset
  ldxh r6, [r7+INSTRUCTION_OFFSET]
  // Get size of ix data
  // Subtract 10 for last_updated_slot (u64) and offset (u16)
  ldxdw r3, [r7+0x0000]
  sub64 r3, 10
  // Prime src for sol_memcpy
  mov64 r2, r7
  add64 r2, INSTRUCTION_UPDATE_DATA
  // Prime dst for sol_memcpy
  add64 r1, MEV_GUARD_BITMAP
  add64 r1, r6
  call sol_memcpy_
success:
  exit
.rodata
  e_account_error: .ascii "Invalid number of accounts"
  e_admin_error: .ascii "Invalid admin address"
  e_blocklisted: .ascii "Blocklisted validator detected"
  e_stale_mev_guard: .ascii "Stale MEV guard data"