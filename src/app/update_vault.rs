use super::*;

impl App {
    pub(super) fn handle_vault_message(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PasswordInputChanged(pw) => { self.password_input = pw; self.auth_error = None; Task::none() }
            Message::ConfirmPasswordInputChanged(pw) => { self.confirm_password_input = pw; self.auth_error = None; Task::none() }
            Message::SubmitSetup => {
                if self.password_input.is_empty() { self.auth_error = Some("Password cannot be empty".into()); return Task::none(); }
                if self.password_input != self.confirm_password_input { self.auth_error = Some("Passwords do not match".into()); return Task::none(); }
                self.vault_state = VaultState::Loading;
                self.loading_tick = 0;
                let password = self.password_input.clone();
                let db_path = self.db_path.clone();
                Task::perform(async move {
                    let salt = crypto::key_derivation::generate_salt();
                    let key = crypto::key_derivation::derive_key(password.as_bytes(), &salt).map_err(|e| e.to_string())?;
                    let (ciphertext, nonce) = crypto::encryption::encrypt(&key.key_bytes, b"notes-app-verify").map_err(|e| e.to_string())?;
                    let conn = db::open_connection(&db_path).map_err(|e| e.to_string())?;
                    db::set_vault_password(&conn, &salt, &nonce, &ciphertext).map_err(|e| e.to_string())?;
                    Ok(key.key_bytes)
                }, Message::SetupDone)
            }
            Message::SetupDone(result) => {
                match result {
                    Ok(key) => { self.vault_state = VaultState::Loading; self.vault_key = Some(key); self.password_input = String::new(); self.confirm_password_input = String::new(); self.auth_error = None; return self.refresh_data(); }
                    Err(e) => { self.vault_state = VaultState::Setup; self.auth_error = Some(e); }
                }
                Task::none()
            }
            Message::SubmitLogin => {
                if self.password_input.is_empty() { self.auth_error = Some("Password cannot be empty".into()); return Task::none(); }
                self.vault_state = VaultState::Loading;
                self.loading_tick = 0;
                let password = self.password_input.clone();
                let db_path = self.db_path.clone();
                Task::perform(async move {
                    let conn = db::open_connection(&db_path).map_err(|e| e.to_string())?;
                    let salt = db::get_vault_salt(&conn).ok_or("No salt found")?;
                    let (nonce, ciphertext) = db::get_vault_verify(&conn).ok_or("No verification data")?;
                    let key = crypto::key_derivation::derive_key(password.as_bytes(), &salt).map_err(|e| e.to_string())?;
                    let plaintext = crypto::encryption::decrypt(&key.key_bytes, &nonce, &ciphertext).map_err(|_| "Wrong password".to_string())?;
                    if plaintext == b"notes-app-verify" { Ok(key.key_bytes) } else { Err("Wrong password".to_string()) }
                }, Message::LoginDone)
            }
            Message::LoginDone(result) => {
                match result {
                    Ok(key) => { self.vault_state = VaultState::Loading; self.vault_key = Some(key); self.password_input = String::new(); self.auth_error = None; return self.refresh_data(); }
                    Err(e) => { self.vault_state = VaultState::Login; self.auth_error = Some(e); }
                }
                Task::none()
            }

            Message::LockVault => {
                let _ = self.maybe_save();
                self.vault_state = VaultState::Login;
                self.vault_key.take().map(|mut k| { k.iter_mut().for_each(|b| *b = 0); });
                self.password_input.clear();
                self.password_input = String::new(); // drop old allocation
                self.selected_note = None;
                self.editor_content = text_editor::Content::new();
                self.line_editor.image_cache.clear();
                self.line_editor.image_sizes.clear();
                self.show_graph = false;
                self.show_settings = false;
                self.session_decrypted.clear();
                Task::none()
            }

            Message::OpenChangeVaultPasswordDialog => {
                self.show_change_password = !self.show_change_password;
                if self.show_change_password {
                    self.vault_old_password.clear();
                    self.vault_new_password.clear();
                    self.vault_new_password_confirm.clear();
                    self.auth_error = None;
                }
                Task::none()
            }
            Message::VaultOldPasswordChanged(pw) => { self.vault_old_password = pw; self.auth_error = None; Task::none() }
            Message::VaultNewPasswordChanged(pw) => { self.vault_new_password = pw; self.auth_error = None; Task::none() }
            Message::VaultNewPasswordConfirmChanged(pw) => { self.vault_new_password_confirm = pw; self.auth_error = None; Task::none() }
            Message::SubmitChangeVaultPassword => {
                if self.vault_old_password.is_empty() { self.auth_error = Some("Enter your current password".into()); return Task::none(); }
                if self.vault_new_password.is_empty() { self.auth_error = Some("New password cannot be empty".into()); return Task::none(); }
                if self.vault_new_password != self.vault_new_password_confirm { self.auth_error = Some("New passwords do not match".into()); return Task::none(); }
                let old_pw = self.vault_old_password.clone();
                let new_pw = self.vault_new_password.clone();
                let db_path = self.db_path.clone();
                Task::perform(async move {
                    let conn = db::open_connection(&db_path).map_err(|e| e.to_string())?;
                    let salt = db::get_vault_salt(&conn).ok_or("No salt found")?;
                    let (nonce, ciphertext) = db::get_vault_verify(&conn).ok_or("No verification data")?;
                    let old_key = crypto::key_derivation::derive_key(old_pw.as_bytes(), &salt).map_err(|e| e.to_string())?;
                    let plaintext = crypto::encryption::decrypt(&old_key.key_bytes, &nonce, &ciphertext).map_err(|_| "Current password is incorrect".to_string())?;
                    if plaintext != b"notes-app-verify" { return Err("Current password is incorrect".to_string()); }
                    let new_salt = crypto::key_derivation::generate_salt();
                    let new_key = crypto::key_derivation::derive_key(new_pw.as_bytes(), &new_salt).map_err(|e| e.to_string())?;
                    let (new_ct, new_nonce) = crypto::encryption::encrypt(&new_key.key_bytes, b"notes-app-verify").map_err(|e| e.to_string())?;
                    db::set_vault_password(&conn, &new_salt, &new_nonce, &new_ct).map_err(|e| e.to_string())?;
                    Ok(())
                }, Message::ChangeVaultPasswordDone)
            }
            Message::ChangeVaultPasswordDone(result) => {
                match result {
                    Ok(()) => {
                        self.vault_old_password.clear();
                        self.vault_new_password.clear();
                        self.vault_new_password_confirm.clear();
                        self.auth_error = None;
                        self.show_change_password = false;
                    }
                    Err(e) => { self.auth_error = Some(e); }
                }
                Task::none()
            }

            _ => Task::none()
        }
    }
}
