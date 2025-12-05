#[cfg(test)]
mod tests {
    use mollusk_svm::{result::Check, Mollusk};
    use solana_svm_log_collector::LogCollector;
    use solana_program_error::ProgramError;

    use solana_account::Account;
    use solana_address::Address;
    use solana_instruction::{Instruction, AccountMeta};
    use solana_rent::Rent;
    use solana_system_interface::instruction::create_account_with_seed;

    const PROGRAM_ID: Address = Address::from_str_const("22222222222222222222222222222222222222222222");
    // 0x1337133713371337133713371337133713371337133713371337133713371337
    const ADMIN: Address = Address::from_str_const("2J1TPM9X3u8DineYw5ANMar41ruY9oQxQ9AMUgdtDD8a");
    const BITMAP_LEN: usize = 13500;

    #[test]
    fn  check_valid_slot_syscall() {
        let mut mollusk = Mollusk::new(&PROGRAM_ID, "deploy/sbpf-asm-mev-guard");

        mollusk.sysvars.clock.epoch = 2;

        println!("{:?}", mollusk.sysvars.clock);

        let (bitmap, bitmap_acccount) = create_keyed_account_for_guard(&ADMIN, &PROGRAM_ID, 2, [0x00u8; BITMAP_LEN]);
    
        let result = mollusk.process_and_validate_instruction(
                &create_check_slot_syscall_ix(&PROGRAM_ID, &bitmap), 
                &[(bitmap, bitmap_acccount)],
                &[Check::success()]
        );
        assert!(!result.program_result.is_err());
    }

    #[test]
    fn  check_invalid_slot_syscall() {
        // 0x200000fc0, 0x0, 0x0, 0x1, 0x350c
        let mut mollusk = Mollusk::new(&PROGRAM_ID, "deploy/sbpf-asm-mev-guard");

        mollusk.sysvars.clock.epoch = 2;

        let logger = Some(LogCollector::new_ref());
        mollusk.logger = logger;

        println!("{:?}", mollusk.sysvars.clock);

        let (bitmap, bitmap_acccount) = create_keyed_account_for_guard(&ADMIN, &PROGRAM_ID, 2, [0xffu8; BITMAP_LEN]);
    
        mollusk.process_and_validate_instruction(
                &create_check_slot_syscall_ix(&PROGRAM_ID, &bitmap),
                &[(bitmap, bitmap_acccount)],
                &[Check::err(ProgramError::InvalidAccountData)]
        );

        let program_logs = match &mollusk.logger {
            Some(log_collector) => log_collector.borrow().messages.clone(),
            None => vec![],
        };

        assert!(program_logs.iter().find(|s| s.starts_with("Program log: Blocklisted validator detected")).is_some());
    }

    #[test]
    fn check_invalid_admin() {
        let (bitmap, bitmap_acccount) = create_keyed_account_for_guard(&ADMIN, &PROGRAM_ID, 1, [0x00u8; BITMAP_LEN]);
    
        let mut mollusk = Mollusk::new(&PROGRAM_ID, "deploy/sbpf-asm-mev-guard");

        let logger = Some(LogCollector::new_ref());
        mollusk.logger = logger;
        
        let fake_admin = Address::new_unique();

        mollusk.process_and_validate_instruction(
                &create_update_mev_guard_ix(&PROGRAM_ID, &fake_admin, &bitmap), 
                &[
                    (bitmap, bitmap_acccount),
                    (fake_admin, Account::new(1_000_000_000, 0, &Address::default()))
                ],
                &[Check::err(ProgramError::IncorrectAuthority)]
        );

        let program_logs = match &mollusk.logger {
            Some(log_collector) => log_collector.borrow().messages.clone(),
            None => vec![],
        };

        assert!(program_logs.iter().find(|s| s.starts_with("Program log: Invalid admin address")).is_some());
    }

    #[test]
    fn check_valid_admin() {
        let (guard, guard_acccount) = create_keyed_account_for_guard(&ADMIN, &PROGRAM_ID, 1, [0x00u8; BITMAP_LEN]);
    
        let mollusk = Mollusk::new(&PROGRAM_ID, "deploy/sbpf-asm-mev-guard");

        let ix = create_update_mev_guard_ix(&PROGRAM_ID, &ADMIN, &guard);

        println!("{:?}", ix);
        
        println!("{:?}", guard_acccount.data.len());


        mollusk.process_and_validate_instruction(
                &ix,
                &[
                    (guard, guard_acccount),
                    (ADMIN, Account::new(1_000_000_000, 0, &Address::default()))
                ],
                &[Check::success()]
        );
    }

    fn create_guard_account_ix(admin: &Address, program_id: &Address, guard: [u8;BITMAP_LEN]) -> Instruction {
        let lamports = Rent::default().minimum_balance(guard.len());
        create_account_with_seed(
            admin,
            &Address::create_with_seed(admin, "guard", program_id).expect("Invalid seed"),
            admin,
            "guard",
            lamports,
            guard.len() as u64,
            program_id,
        )
    }

    fn create_check_slot_syscall_ix(program_id: &Address, guard: &Address) -> Instruction {
        Instruction::new_with_bytes(
            *program_id, 
            &[],
            vec![
                AccountMeta::new_readonly(*guard, false)
            ]
        )
    }

    fn create_update_mev_guard_ix(program_id: &Address, admin: &Address, guard: &Address) -> Instruction {
        Instruction::new_with_bytes(
            *program_id, 
            &[
                1u64.to_le_bytes().as_ref(),
                10u16.to_le_bytes().as_ref(),
                [0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01].as_ref()
            ].concat(),
            vec![
                AccountMeta::new(*guard, false),
                AccountMeta::new_readonly(*admin, true)
            ]
        )
    }

    fn create_keyed_account_for_guard(admin: &Address, program_id: &Address, slot: u64, guard: [u8;BITMAP_LEN]) -> (Address, Account) {
        let address = Address::create_with_seed(admin, "guard", program_id).expect("Invalid seed");
        let data = [
            slot.to_le_bytes().as_ref(),
            guard.as_ref()
        ].concat();
        let lamports = Rent::default().minimum_balance(data.len());
        let mut account = Account::new(lamports, data.len(), program_id);
        account.data = data;
        (address, account)
    }
}

