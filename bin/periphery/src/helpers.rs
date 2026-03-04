use anyhow::Context;
use komodo_client::{
  entities::{EnvironmentVar, RepoExecutionArgs, SearchCombinator},
  parsers::QUOTE_PATTERN,
};

use crate::config::periphery_config;

pub fn git_token_simple(
  domain: &str,
  account_username: &str,
) -> anyhow::Result<&'static str> {
  periphery_config()
    .git_providers
    .iter()
    .find(|provider| provider.domain == domain)
    .and_then(|provider| {
      provider.accounts.iter().find(|account| account.username == account_username).map(|account| account.token.as_str())
    })
    .with_context(|| format!("Did not find token in config for git account {account_username} | domain {domain}"))
}

pub fn git_token(
  core_token: Option<String>,
  args: &RepoExecutionArgs,
) -> anyhow::Result<Option<String>> {
  if core_token.is_some() {
    return Ok(core_token);
  }
  let Some(account) = &args.account else {
    return Ok(None);
  };
  let token = git_token_simple(&args.provider, account)?;
  Ok(Some(token.to_string()))
}

pub fn registry_token(
  domain: &str,
  account_username: &str,
) -> anyhow::Result<&'static str> {
  periphery_config()
    .docker_registries
    .iter()
    .find(|registry| registry.domain == domain)
    .and_then(|registry| {
      registry.accounts.iter().find(|account| account.username == account_username).map(|account| account.token.as_str())
    })
    .with_context(|| format!("did not find token in config for docker registry account {account_username} | domain {domain}"))
}

pub fn parse_extra_args(extra_args: &[String]) -> String {
  let args = extra_args.join(" ");
  if !args.is_empty() {
    format!(" {args}")
  } else {
    args
  }
}

pub fn parse_labels(labels: &[EnvironmentVar]) -> String {
  labels
    .iter()
    .map(|p| {
      if p.value.starts_with(QUOTE_PATTERN)
        && p.value.ends_with(QUOTE_PATTERN)
      {
        if p.value.starts_with('\'') {
          // Single-quoted: backticks are literal, pass through as-is
          format!(" --label {}={}", p.variable, p.value)
        } else if let Some(inner) = p
          .value
          .strip_prefix('"')
          .and_then(|s| s.strip_suffix('"'))
        {
          // Double-quoted: escape backticks to prevent shell
          // command substitution (needed for e.g. Traefik Host() rules)
          format!(
            " --label {}=\"{}\"",
            p.variable,
            inner.replace('`', "\\`")
          )
        } else {
          // Mismatched quotes: pass through as-is
          format!(" --label {}={}", p.variable, p.value)
        }
      } else {
        // Escape backticks to prevent shell command substitution
        // when value is wrapped in double quotes
        format!(
          " --label {}=\"{}\"",
          p.variable,
          p.value.replace('`', "\\`")
        )
      }
    })
    .collect::<Vec<_>>()
    .join("")
}

pub fn log_grep(
  terms: &[String],
  combinator: SearchCombinator,
  invert: bool,
) -> String {
  let maybe_invert = if invert { " -v" } else { Default::default() };
  match combinator {
    SearchCombinator::Or => {
      format!("grep{maybe_invert} -E '{}'", terms.join("|"))
    }
    SearchCombinator::And => {
      format!(
        "grep{maybe_invert} -P '^(?=.*{})'",
        terms.join(")(?=.*")
      )
    }
  }
}
