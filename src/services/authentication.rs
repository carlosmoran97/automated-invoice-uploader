use serde::Deserialize;
use serde_json::json;
use std::{
    fmt,
    io::{self, Read},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const CHECK_TIMEOUT: Duration = Duration::from_secs(30);
const REQUIRED_SCOPES: &str =
    "https://www.googleapis.com/auth/gmail.readonly,https://www.googleapis.com/auth/drive";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthenticationCheck {
    Status,
    Gmail,
    Drive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginReason {
    MissingCredentials,
    ExpiredSession,
    InsufficientScopes,
    UnreadableCredentials,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthenticationEvent {
    Checking,
    LoginRequired(LoginReason),
    Verifying,
    Ready,
}

#[derive(Debug)]
pub enum AuthenticationError {
    CliNotFound,
    Timeout(AuthenticationCheck),
    Io(io::Error),
    InvalidResponse(AuthenticationCheck),
    Configuration,
    Connection(AuthenticationCheck),
    ApiDisabled(AuthenticationCheck),
    PermissionDenied(AuthenticationCheck),
    CheckFailed(AuthenticationCheck),
    LoginRequired(LoginReason),
    LoginFailed,
}

impl fmt::Display for AuthenticationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Authentication failed: {self:?}")
    }
}

impl std::error::Error for AuthenticationError {}

#[derive(Debug)]
pub struct CommandOutput {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug)]
pub enum CommandError {
    CliNotFound,
    Timeout,
    Io(io::Error),
}

pub trait CommandRunner {
    fn capture(&self, args: &[&str], timeout: Duration) -> Result<CommandOutput, CommandError>;
    fn login(&self, args: &[&str]) -> Result<bool, CommandError>;
}

#[derive(Default)]
pub struct SystemCommandRunner;

pub struct AuthenticationService<R = SystemCommandRunner> {
    runner: R,
}

impl Default for AuthenticationService<SystemCommandRunner> {
    fn default() -> Self {
        Self {
            runner: SystemCommandRunner,
        }
    }
}

impl<R: CommandRunner> AuthenticationService<R> {
    pub fn ensure_authenticated(
        &self,
        mut notify: impl FnMut(AuthenticationEvent),
    ) -> Result<(), AuthenticationError> {
        notify(AuthenticationEvent::Checking);
        match self.check_access() {
            Ok(()) => {}
            Err(AuthenticationError::LoginRequired(reason)) => {
                notify(AuthenticationEvent::LoginRequired(reason));
                let success = self
                    .runner
                    .login(&["auth", "login", "--scopes", REQUIRED_SCOPES])
                    .map_err(|error| command_error(error, AuthenticationCheck::Status))?;
                if !success {
                    return Err(AuthenticationError::LoginFailed);
                }
                notify(AuthenticationEvent::Verifying);
                self.check_access()?;
            }
            Err(error) => return Err(error),
        }
        notify(AuthenticationEvent::Ready);
        Ok(())
    }

    fn check_access(&self) -> Result<(), AuthenticationError> {
        let output = self.capture(AuthenticationCheck::Status, &["auth", "status"])?;
        let status: AuthStatus = serde_json::from_value(output)
            .map_err(|_| AuthenticationError::InvalidResponse(AuthenticationCheck::Status))?;

        // Live probes also honor credential sources that auth status may not report.
        let gmail = self.capture(
            AuthenticationCheck::Gmail,
            &[
                "gmail",
                "users",
                "messages",
                "list",
                "--params",
                &json!({
                    "userId": "me",
                    "q": "in:inbox",
                    "maxResults": 1,
                    "fields": "resultSizeEstimate",
                })
                .to_string(),
            ],
        );
        let drive = self.capture(
            AuthenticationCheck::Drive,
            &[
                "drive",
                "files",
                "list",
                "--params",
                &json!({"pageSize": 1, "fields": "kind"}).to_string(),
            ],
        );

        let mut login_reason = None;
        for result in [gmail, drive] {
            match result {
                Ok(_) => {}
                Err(AuthenticationError::LoginRequired(reason)) => {
                    login_reason = Some(reason);
                }
                Err(error) => return Err(error),
            }
        }

        if login_reason.is_none()
            && !status.token_env_var
            && status.token_valid == Some(false)
            && let Some(error) = &status.token_error
            && let AuthenticationError::LoginRequired(reason) = classify_failure(
                AuthenticationCheck::Status,
                &json!({"code": 401, "reason": "authError", "message": error}),
            )
        {
            login_reason = Some(reason);
        }

        // Read-only API probes cannot prove that Drive writes were authorized.
        if !status.token_env_var
            && status.auth_method == "oauth2"
            && status.token_valid == Some(true)
            && let Some(scopes) = &status.scopes
        {
            let gmail_read = scopes.iter().any(|scope| {
                matches!(
                    scope.as_str(),
                    "https://www.googleapis.com/auth/gmail.readonly"
                        | "https://www.googleapis.com/auth/gmail.modify"
                        | "https://mail.google.com/"
                )
            });
            let drive_write = scopes
                .iter()
                .any(|scope| scope == "https://www.googleapis.com/auth/drive");
            if !gmail_read || !drive_write {
                login_reason = Some(LoginReason::InsufficientScopes);
            }
        }

        if let Some(reason) = login_reason {
            if status.client_config_error.is_some()
                || (status.client_config_exists == Some(false)
                    && status.credential_source.as_deref() == Some("none"))
            {
                return Err(AuthenticationError::Configuration);
            }
            let reason = if status.encryption_valid == Some(false) {
                LoginReason::UnreadableCredentials
            } else if status.auth_method == "none" && reason == LoginReason::ExpiredSession {
                LoginReason::MissingCredentials
            } else {
                reason
            };
            return Err(AuthenticationError::LoginRequired(reason));
        }

        Ok(())
    }

    fn capture(
        &self,
        check: AuthenticationCheck,
        args: &[&str],
    ) -> Result<serde_json::Value, AuthenticationError> {
        let output = self
            .runner
            .capture(args, CHECK_TIMEOUT)
            .map_err(|error| command_error(error, check))?;
        let body = serde_json::from_str::<serde_json::Value>(&output.stdout);
        if let Ok(body) = &body
            && let Some(error) = body.get("error")
        {
            return Err(classify_failure(check, error));
        }
        if output.code != Some(0) {
            if let Ok(body) = serde_json::from_str::<serde_json::Value>(&output.stderr)
                && let Some(error) = body.get("error")
            {
                return Err(classify_failure(check, error));
            }
            return Err(AuthenticationError::CheckFailed(check));
        }
        body.ok()
            .filter(serde_json::Value::is_object)
            .ok_or(AuthenticationError::InvalidResponse(check))
    }
}

#[derive(Deserialize)]
struct AuthStatus {
    auth_method: String,
    #[serde(default)]
    token_env_var: bool,
    token_valid: Option<bool>,
    token_error: Option<String>,
    encryption_valid: Option<bool>,
    scopes: Option<Vec<String>>,
    client_config_exists: Option<bool>,
    client_config_error: Option<String>,
    credential_source: Option<String>,
}

fn classify_failure(check: AuthenticationCheck, error: &serde_json::Value) -> AuthenticationError {
    let reason = error["reason"].as_str().unwrap_or_default();
    let message = error["message"].as_str().unwrap_or_default().to_lowercase();
    let code = error["code"].as_u64();
    let contains_any = |needles: &[&str]| needles.iter().any(|needle| message.contains(needle));

    // gws can wrap transport and OAuth configuration failures as authError/401.
    if contains_any(&[
        "error sending request",
        "failed to send",
        "connection",
        "connect error",
        "dns",
        "name resolution",
        "timed out",
        "timeout",
        "network",
        "tls",
        "certificate",
        "temporarily unavailable",
        "temporarily_unavailable",
    ]) {
        return AuthenticationError::Connection(check);
    }
    if contains_any(&[
        "invalid_client",
        "invalid client",
        "unauthorized_client",
        "invalid_scope",
        "client_secret",
        "client secret",
        "client config",
        "oauth client",
        "redirect_uri_mismatch",
        "failed to parse",
        "failed to read credentials",
        "file does not exist",
    ]) || reason == "validationError"
    {
        return AuthenticationError::Configuration;
    }
    if matches!(reason, "accessNotConfigured" | "serviceDisabled") {
        return AuthenticationError::ApiDisabled(check);
    }
    if contains_any(&["admin_policy_enforced", "access_denied"])
        || matches!(reason, "domainPolicy" | "adminPolicyEnforced")
    {
        return AuthenticationError::PermissionDenied(check);
    }
    let login_reason = if reason == "insufficientPermissions"
        || contains_any(&[
            "insufficient authentication scopes",
            "access_token_scope_insufficient",
        ]) {
        LoginReason::InsufficientScopes
    } else if contains_any(&["decrypt", "encryption key"]) {
        LoginReason::UnreadableCredentials
    } else if contains_any(&[
        "no credentials found",
        "credentials missing",
        "not authenticated",
    ]) {
        LoginReason::MissingCredentials
    } else if contains_any(&[
        "invalid_grant",
        "expired or revoked",
        "token expired",
        "token has expired",
        "token has been expired",
        "invalid credentials",
        "invalid authentication credentials",
    ]) || (code == Some(401) && reason != "authError")
    {
        LoginReason::ExpiredSession
    } else if code == Some(403) {
        return AuthenticationError::PermissionDenied(check);
    } else {
        return AuthenticationError::CheckFailed(check);
    };
    AuthenticationError::LoginRequired(login_reason)
}

fn command_error(error: CommandError, check: AuthenticationCheck) -> AuthenticationError {
    match error {
        CommandError::CliNotFound => AuthenticationError::CliNotFound,
        CommandError::Timeout => AuthenticationError::Timeout(check),
        CommandError::Io(error) => AuthenticationError::Io(error),
    }
}

fn spawn_error(error: io::Error) -> CommandError {
    if error.kind() == io::ErrorKind::NotFound {
        CommandError::CliNotFound
    } else {
        CommandError::Io(error)
    }
}

impl CommandRunner for SystemCommandRunner {
    fn capture(&self, args: &[&str], timeout: Duration) -> Result<CommandOutput, CommandError> {
        capture_command(Command::new("gws").args(args), timeout)
    }

    fn login(&self, args: &[&str]) -> Result<bool, CommandError> {
        Command::new("gws")
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .map_err(spawn_error)
    }
}

fn read_pipe(mut pipe: impl Read + Send + 'static) -> mpsc::Receiver<io::Result<Vec<u8>>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = pipe.read_to_end(&mut bytes).map(|_| bytes);
        let _ = sender.send(result);
    });
    receiver
}

fn pipe_output(
    receiver: mpsc::Receiver<io::Result<Vec<u8>>>,
    deadline: Instant,
) -> Result<String, CommandError> {
    match receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(Ok(bytes)) => Ok(String::from_utf8_lossy(&bytes).into_owned()),
        Ok(Err(error)) => Err(CommandError::Io(error)),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(CommandError::Timeout),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(CommandError::Io(io::Error::other(
            "Could not read gws command output",
        ))),
    }
}

fn capture_command(
    command: &mut Command,
    timeout: Duration,
) -> Result<CommandOutput, CommandError> {
    let deadline = Instant::now() + timeout;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(spawn_error)?;
    // Drain both pipes while waiting so verbose subprocesses cannot block on a full pipe.
    let stdout = read_pipe(child.stdout.take().expect("stdout is piped"));
    let stderr = read_pipe(child.stderr.take().expect("stderr is piped"));
    let result = (|| {
        let status = loop {
            if Instant::now() >= deadline {
                return Err(CommandError::Timeout);
            }
            if let Some(status) = child.try_wait().map_err(CommandError::Io)? {
                break status;
            }
            thread::sleep(
                Duration::from_millis(20).min(deadline.saturating_duration_since(Instant::now())),
            );
        };
        Ok(CommandOutput {
            code: status.code(),
            stdout: pipe_output(stdout, deadline)?,
            stderr: pipe_output(stderr, deadline)?,
        })
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::VecDeque};

    const VALID_STATUS: &str = r#"{
        "auth_method":"oauth2", "token_valid":true, "encryption_valid":true,
        "scopes":["https://www.googleapis.com/auth/gmail.readonly", "https://www.googleapis.com/auth/drive"]
    }"#;

    enum Step {
        Capture(Result<CommandOutput, CommandError>),
        Login(bool),
    }

    struct FakeRunner {
        steps: RefCell<VecDeque<Step>>,
        calls: RefCell<Vec<Vec<String>>>,
    }

    impl CommandRunner for FakeRunner {
        fn capture(&self, args: &[&str], timeout: Duration) -> Result<CommandOutput, CommandError> {
            assert_eq!(timeout, Duration::from_secs(30));
            self.calls
                .borrow_mut()
                .push(args.iter().map(|arg| arg.to_string()).collect());
            match self
                .steps
                .borrow_mut()
                .pop_front()
                .expect("missing response")
            {
                Step::Capture(result) => result,
                Step::Login(_) => panic!("expected login instead of capture"),
            }
        }

        fn login(&self, args: &[&str]) -> Result<bool, CommandError> {
            self.calls
                .borrow_mut()
                .push(args.iter().map(|arg| arg.to_string()).collect());
            match self
                .steps
                .borrow_mut()
                .pop_front()
                .expect("missing response")
            {
                Step::Login(success) => Ok(success),
                Step::Capture(_) => panic!("expected capture instead of login"),
            }
        }
    }

    fn service(steps: Vec<Step>) -> AuthenticationService<FakeRunner> {
        AuthenticationService {
            runner: FakeRunner {
                steps: RefCell::new(steps.into()),
                calls: RefCell::new(Vec::new()),
            },
        }
    }

    fn output(body: &str) -> Step {
        Step::Capture(Ok(CommandOutput {
            code: Some(0),
            stdout: body.to_string(),
            stderr: String::new(),
        }))
    }

    fn failure(code: u16, reason: &str, message: &str) -> Step {
        Step::Capture(Ok(CommandOutput {
            code: Some(1),
            stdout: json!({"error": {"code": code, "reason": reason, "message": message}})
                .to_string(),
            stderr: String::new(),
        }))
    }

    fn valid_steps() -> Vec<Step> {
        vec![
            output(VALID_STATUS),
            output(r#"{"resultSizeEstimate":0}"#),
            output(r#"{"kind":"drive#fileList"}"#),
        ]
    }

    fn login_count(service: &AuthenticationService<FakeRunner>) -> usize {
        service
            .runner
            .calls
            .borrow()
            .iter()
            .filter(|args| args.get(1).map(String::as_str) == Some("login"))
            .count()
    }

    #[test]
    fn valid_session_checks_both_apis_without_login_or_private_data() {
        let service = service(valid_steps());
        let mut events = Vec::new();
        service
            .ensure_authenticated(|event| events.push(event))
            .unwrap();
        assert_eq!(
            events,
            vec![AuthenticationEvent::Checking, AuthenticationEvent::Ready]
        );
        let calls = service.runner.calls.borrow();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0], ["auth", "status"]);
        assert_eq!(&calls[1][..4], ["gmail", "users", "messages", "list"]);
        let gmail: serde_json::Value = serde_json::from_str(&calls[1][5]).unwrap();
        assert_eq!(gmail["maxResults"], 1);
        assert_eq!(gmail["fields"], "resultSizeEstimate");
        assert_eq!(gmail["q"], "in:inbox");
        assert_eq!(&calls[2][..3], ["drive", "files", "list"]);
        let drive: serde_json::Value = serde_json::from_str(&calls[2][4]).unwrap();
        assert_eq!(drive["pageSize"], 1);
        assert_eq!(drive["fields"], "kind");
        assert!(service.runner.steps.borrow().is_empty());
    }

    #[test]
    fn successful_probes_allow_gws_to_refresh_without_interactive_login() {
        let mut steps = valid_steps();
        steps[0] = output(r#"{"auth_method":"oauth2", "has_refresh_token":true}"#);
        let service = service(steps);
        service.ensure_authenticated(|_| {}).unwrap();
        assert_eq!(login_count(&service), 0);
    }

    #[test]
    fn successful_probes_allow_credentials_from_other_gws_sources() {
        for status in [
            r#"{"auth_method":"none"}"#,
            r#"{"auth_method":"oauth2", "encryption_valid":false}"#,
            r#"{"auth_method":"oauth2", "token_env_var":true, "token_valid":true, "scopes":[]}"#,
            r#"{"auth_method":"oauth2", "token_env_var":true, "token_valid":false, "token_error":"Token has been expired or revoked."}"#,
        ] {
            let mut steps = valid_steps();
            steps[0] = output(status);
            let service = service(steps);
            service.ensure_authenticated(|_| {}).unwrap();
            assert_eq!(login_count(&service), 0);
        }
    }

    #[test]
    fn missing_revoked_insufficient_and_unreadable_credentials_login_once() {
        for (status, code, reason, message, expected) in [
            (
                r#"{"auth_method":"none"}"#,
                401,
                "authError",
                "No credentials found. Run gws auth login.",
                LoginReason::MissingCredentials,
            ),
            (
                r#"{"auth_method":"oauth2", "token_valid":false}"#,
                401,
                "authError",
                "Failed to get token: invalid_grant: Token has been expired or revoked.",
                LoginReason::ExpiredSession,
            ),
            (
                VALID_STATUS,
                403,
                "insufficientPermissions",
                "Request had insufficient authentication scopes.",
                LoginReason::InsufficientScopes,
            ),
            (
                r#"{"auth_method":"oauth2", "encryption_valid":false}"#,
                401,
                "authError",
                "No credentials found. Run gws auth login.",
                LoginReason::UnreadableCredentials,
            ),
        ] {
            let mut steps = vec![
                output(status),
                failure(code, reason, message),
                failure(code, reason, message),
                Step::Login(true),
            ];
            steps.extend(valid_steps());
            let service = service(steps);
            let mut events = Vec::new();
            service
                .ensure_authenticated(|event| events.push(event))
                .unwrap();
            assert_eq!(
                events,
                vec![
                    AuthenticationEvent::Checking,
                    AuthenticationEvent::LoginRequired(expected),
                    AuthenticationEvent::Verifying,
                    AuthenticationEvent::Ready,
                ]
            );
            assert_eq!(login_count(&service), 1);
            let calls = service.runner.calls.borrow();
            assert_eq!(
                calls[3],
                [
                    "auth",
                    "login",
                    "--scopes",
                    "https://www.googleapis.com/auth/gmail.readonly,https://www.googleapis.com/auth/drive"
                ]
            );
            assert!(service.runner.steps.borrow().is_empty());
        }
    }

    #[test]
    fn revoked_refresh_token_requires_login_even_when_cached_api_tokens_work() {
        let mut steps = valid_steps();
        steps[0] = output(
            r#"{"auth_method":"oauth2", "token_valid":false, "token_error":"Token has been expired or revoked."}"#,
        );
        steps.push(Step::Login(true));
        steps.extend(valid_steps());
        let service = service(steps);
        service.ensure_authenticated(|_| {}).unwrap();
        assert_eq!(login_count(&service), 1);
    }

    #[test]
    fn read_only_drive_scope_requires_login_even_when_both_probes_work() {
        let mut steps = valid_steps();
        steps[0] = output(
            r#"{"auth_method":"oauth2", "token_valid":true, "scopes":["https://www.googleapis.com/auth/gmail.readonly", "https://www.googleapis.com/auth/drive.readonly"]}"#,
        );
        steps.push(Step::Login(true));
        steps.extend(valid_steps());
        let service = service(steps);
        service.ensure_authenticated(|_| {}).unwrap();
        assert_eq!(login_count(&service), 1);
    }

    #[test]
    fn existing_broader_gmail_permissions_do_not_require_login() {
        for scope in [
            "https://www.googleapis.com/auth/gmail.modify",
            "https://mail.google.com/",
        ] {
            let mut steps = valid_steps();
            steps[0] = output(
                &json!({
                    "auth_method": "oauth2", "token_valid": true,
                    "scopes": [scope, "https://www.googleapis.com/auth/drive"],
                })
                .to_string(),
            );
            let service = service(steps);
            service.ensure_authenticated(|_| {}).unwrap();
            assert_eq!(login_count(&service), 0);
        }
    }

    #[test]
    fn cancelled_login_stops_before_rechecking_or_opening_the_interface() {
        let service = service(vec![
            output(r#"{"auth_method":"none"}"#),
            failure(401, "authError", "No credentials found"),
            failure(401, "authError", "No credentials found"),
            Step::Login(false),
        ]);
        let mut events = Vec::new();
        let result = service.ensure_authenticated(|event| events.push(event));
        assert!(matches!(result, Err(AuthenticationError::LoginFailed)));
        assert_eq!(login_count(&service), 1);
        assert!(!events.contains(&AuthenticationEvent::Ready));
        assert!(!events.contains(&AuthenticationEvent::Verifying));
        assert!(service.runner.steps.borrow().is_empty());
    }

    #[test]
    fn failed_verification_never_runs_a_second_login() {
        let service = service(vec![
            output(VALID_STATUS),
            failure(401, "authError", "Invalid Credentials"),
            output("{}"),
            Step::Login(true),
            output(VALID_STATUS),
            failure(401, "authError", "Invalid Credentials"),
            output("{}"),
        ]);
        let mut events = Vec::new();
        let result = service.ensure_authenticated(|event| events.push(event));
        assert!(matches!(
            result,
            Err(AuthenticationError::LoginRequired(
                LoginReason::ExpiredSession
            ))
        ));
        assert_eq!(login_count(&service), 1);
        assert!(!events.contains(&AuthenticationEvent::Ready));
        assert!(service.runner.steps.borrow().is_empty());
    }

    #[test]
    fn transport_configuration_policy_and_api_failures_do_not_trigger_login() {
        for (code, reason, message) in [
            (
                401,
                "authError",
                "Failed to get token: error sending request: DNS error",
            ),
            (401, "authError", "invalid_client: Unauthorized"),
            (401, "authError", "invalid_scope"),
            (403, "accessNotConfigured", "API has not been enabled"),
            (403, "serviceDisabled", "API disabled"),
            (403, "domainPolicy", "Forbidden by policy"),
            (401, "authError", "admin_policy_enforced"),
            (403, "forbidden", "Forbidden"),
            (429, "rateLimitExceeded", "Too many requests"),
            (503, "backendError", "Service unavailable"),
            (500, "discoveryError", "Failed to fetch schema"),
            (
                401,
                "authError",
                "Failed to build authorized user authenticator",
            ),
        ] {
            let service = service(vec![
                output(VALID_STATUS),
                failure(code, reason, message),
                output("{}"),
            ]);
            let mut events = Vec::new();
            let result = service.ensure_authenticated(|event| events.push(event));
            assert!(result.is_err(), "{reason}: {message}");
            assert!(!matches!(
                result,
                Err(AuthenticationError::LoginRequired(_))
            ));
            assert_eq!(login_count(&service), 0);
            assert!(!events.contains(&AuthenticationEvent::Ready));
        }
    }

    #[test]
    fn network_failure_takes_priority_over_missing_credentials() {
        let service = service(vec![
            output(r#"{"auth_method":"none"}"#),
            failure(401, "authError", "No credentials found"),
            failure(401, "authError", "Failed to send token refresh request"),
        ]);
        let result = service.ensure_authenticated(|_| {});
        assert!(matches!(
            result,
            Err(AuthenticationError::Connection(AuthenticationCheck::Drive))
        ));
        assert_eq!(login_count(&service), 0);
    }

    #[test]
    fn invalid_or_missing_oauth_client_stops_without_login() {
        for status in [
            r#"{"auth_method":"none", "client_config_exists":false, "credential_source":"none"}"#,
            r#"{"auth_method":"oauth2", "client_config_error":"Invalid client JSON"}"#,
        ] {
            let service = service(vec![
                output(status),
                failure(401, "authError", "No credentials found"),
                output("{}"),
            ]);
            let result = service.ensure_authenticated(|_| {});
            assert!(matches!(result, Err(AuthenticationError::Configuration)));
            assert_eq!(login_count(&service), 0);
        }
    }

    #[test]
    fn missing_oauth_client_does_not_attempt_a_scope_upgrade() {
        let mut steps = valid_steps();
        steps[0] = output(
            r#"{"auth_method":"oauth2", "token_valid":true, "scopes":[], "client_config_exists":false, "credential_source":"none"}"#,
        );
        let service = service(steps);
        assert!(matches!(
            service.ensure_authenticated(|_| {}),
            Err(AuthenticationError::Configuration)
        ));
        assert_eq!(login_count(&service), 0);
    }

    #[test]
    fn invalid_status_json_or_shape_stops_without_login() {
        for body in ["not JSON", "{}", "[]", r#"{"auth_method":false}"#] {
            let service = service(vec![output(body)]);
            let result = service.ensure_authenticated(|_| {});
            assert!(matches!(
                result,
                Err(AuthenticationError::InvalidResponse(
                    AuthenticationCheck::Status
                ))
            ));
            assert_eq!(login_count(&service), 0);
        }
    }

    #[test]
    fn malformed_probe_response_stops_without_login() {
        let service = service(vec![output(VALID_STATUS), output("[]"), output("{}")]);
        let result = service.ensure_authenticated(|_| {});
        assert!(matches!(
            result,
            Err(AuthenticationError::InvalidResponse(
                AuthenticationCheck::Gmail
            ))
        ));
        assert_eq!(login_count(&service), 0);
    }

    #[test]
    fn missing_cli_and_timeout_stop_without_login() {
        for error in [CommandError::CliNotFound, CommandError::Timeout] {
            let service = service(vec![Step::Capture(Err(error))]);
            let result = service.ensure_authenticated(|_| {});
            assert!(matches!(
                result,
                Err(AuthenticationError::CliNotFound)
                    | Err(AuthenticationError::Timeout(AuthenticationCheck::Status))
            ));
            assert_eq!(login_count(&service), 0);
        }
    }

    #[test]
    fn probe_timeout_does_not_start_login() {
        let service = service(vec![
            output(VALID_STATUS),
            output("{}"),
            Step::Capture(Err(CommandError::Timeout)),
        ]);
        let result = service.ensure_authenticated(|_| {});
        assert!(matches!(
            result,
            Err(AuthenticationError::Timeout(AuthenticationCheck::Drive))
        ));
        assert_eq!(login_count(&service), 0);
    }

    #[test]
    fn nonzero_status_uses_structured_stderr_errors() {
        let service = service(vec![Step::Capture(Ok(CommandOutput {
            code: Some(2),
            stdout: String::new(),
            stderr: json!({"error": {
                "code": 401, "reason": "authError", "message": "invalid_client"
            }})
            .to_string(),
        }))]);
        assert!(matches!(
            service.ensure_authenticated(|_| {}),
            Err(AuthenticationError::Configuration)
        ));
        assert_eq!(login_count(&service), 0);
    }

    #[test]
    fn successful_exit_with_an_error_body_is_not_authenticated() {
        let service = service(vec![output(
            r#"{"error":{"code":403,"reason":"accessNotConfigured","message":"API disabled"}}"#,
        )]);
        assert!(matches!(
            service.ensure_authenticated(|_| {}),
            Err(AuthenticationError::ApiDisabled(
                AuthenticationCheck::Status
            ))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn capture_drains_both_pipes_without_deadlocking() {
        let result = capture_command(
            Command::new("sh").args([
                "-c",
                "head -c 131072 /dev/zero; head -c 131072 /dev/zero >&2",
            ]),
            Duration::from_secs(5),
        )
        .unwrap();
        assert_eq!(result.code, Some(0));
        assert_eq!(result.stdout.len(), 131072);
        assert_eq!(result.stderr.len(), 131072);
    }

    #[cfg(unix)]
    #[test]
    fn capture_times_out_and_reaps_a_hung_process() {
        let start = Instant::now();
        let result = capture_command(
            Command::new("sh").args(["-c", "exec sleep 5"]),
            Duration::from_millis(50),
        );
        assert!(matches!(result, Err(CommandError::Timeout)));
        assert!(start.elapsed() < Duration::from_secs(2));
    }
}
