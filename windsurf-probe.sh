#!/usr/bin/env bash
set -euo pipefail

ROOT_TS="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"

BRIEF="${BRIEF:-1}"

redact() {
  perl -pe '
    s/([A-Za-z_]*(API[_-]?KEY|TOKEN|SECRET|PASSWORD)[A-Za-z_]*\s*[:=]\s*)(\S+)/$1<REDACTED>/ig;
    s/\b(sk-[A-Za-z0-9]{16,})\b/<REDACTED>/g;
    s/\b([A-Za-z0-9_\-]{24,})\b/<REDACTED>/g;
  '
}

section() {
  echo
  echo "==== $1 ===="
}

run() {
  local title="$1"; shift
  section "$title"
  ("$@" 2>&1 || true) | redact
}

print_kv() {
  printf '%-24s %s\n' "$1" "$2"
}

section "Context"
print_kv "timestamp_utc" "$ROOT_TS"
print_kv "whoami" "$(whoami)"
print_kv "uname" "$(uname -a)"
print_kv "windsurf_app" "/Applications/Windsurf.app"
print_kv "windsurf_support" "$HOME/Library/Application Support/Windsurf"

section "Process: Windsurf"
ps axo pid,ppid,etime,command | grep -i "[W]indsurf" | redact || true

section "PIDs"
# Prefer ps-based detection to avoid matching the current shell command while still
# catching helper processes (Renderer/Plugin/GPU) under the app bundle.
PIDS="$(
  ps ax -o pid= -o command= \
    | grep -E '/Applications/Windsurf\.app/' \
    | grep -v 'windsurf-probe\.sh' \
    | awk '{print $1}' \
    | sort -u \
    | tr '\n' ' '
)"
if [[ -z "${PIDS}" ]]; then
  echo "No Windsurf process found (ps | /Applications/Windsurf.app/)."
else
  echo "${PIDS}" | tr ' ' '\n'
fi

if [[ "${BRIEF}" != "1" ]]; then
  section "Process command lines (Windsurf)"
  if [[ -n "${PIDS}" ]]; then
    for pid in ${PIDS}; do
      echo "-- pid=${pid}"
      (ps -p "${pid}" -o command= 2>&1 || true) | redact
      # Surface potential code locations without dumping everything
      (lsof -nP -p "${pid}" 2>/dev/null | grep -E "(node_modules\.asar|extension\.js|windsurf/dist/extension\.js|Resources/app|app/out)" | head -n 40 || true) | redact
    done
  fi
fi

if [[ "${BRIEF}" != "1" ]]; then
  if [[ -n "${PIDS}" ]]; then
    for pid in ${PIDS}; do
      run "lsof LISTEN ports for pid=${pid}" lsof -nP -p "${pid}" -iTCP -sTCP:LISTEN
    done
  fi
fi

section "Windsurf LISTEN port details"
if [[ -n "${PIDS}" ]]; then
  PORTS="$(lsof -nP -iTCP -sTCP:LISTEN 2>/dev/null | awk '/Windsurf/ {print $9}' | sed -E 's/.*:([0-9]+)$/\1/' | sort -n | uniq | tr '\n' ' ')"
  if [[ -z "${PORTS}" ]]; then
    echo "No LISTEN ports matched to process name 'Windsurf' via lsof."
  else
    echo "ports: ${PORTS}" | redact
    for port in ${PORTS}; do
      run "Listener details for TCP:${port}" lsof -nP -iTCP:"${port}" -sTCP:LISTEN
    done

    if command -v curl >/dev/null 2>&1; then
      for port in ${PORTS}; do
        section "HTTP probe (best-effort) for 127.0.0.1:${port}"
        (curl --noproxy "*" -sS -I --max-time 2 "http://127.0.0.1:${port}/" 2>&1 || true) | head -n 40 | redact
        (curl --noproxy "*" -sS --max-time 2 "http://127.0.0.1:${port}/health" 2>&1 || true) | head -n 40 | redact

        section "HTTP path probe (best-effort) for 127.0.0.1:${port}"
        paths=(
          "/api"
          "/api/health"
          "/status"
          "/version"
          "/mcp"
          "/.well-known/agent-card.json"
          "/v1"
          "/v1/models"
          "/v1/chat/completions"
          "/chat/completions"
          "/openai"
          "/anthropic"
          "/claude"
          "/codex"
          "/completion"
          "/completions"
          "/tasks"
          "/executeCommand"
          "/listCommands"
        )

        for path in "${paths[@]}"; do
          code="$(curl --noproxy "*" -sS -o /dev/null -w '%{http_code}' --max-time 2 "http://127.0.0.1:${port}${path}" 2>/dev/null || true)"
          if [[ -n "${code}" && "${code}" != "000" ]]; then
            printf '%-28s %s\n' "${path}" "${code}" | redact
          fi
        done
      done
    else
      echo "curl not found; skipping HTTP probe."
    fi
  fi
fi

if [[ "${BRIEF}" != "1" ]]; then
  run "Global LISTEN sockets (filtered: windsurf)" bash -lc "lsof -nP -iTCP -sTCP:LISTEN | grep -i windsurf"
  run "Global LISTEN sockets (top 80)" bash -lc "lsof -nP -iTCP -sTCP:LISTEN | head -n 80"

  section "Network connections (filtered: windsurf)"
  if [[ -n "${PIDS}" ]]; then
    for pid in ${PIDS}; do
      run "lsof TCP connections for pid=${pid} (first 120)" bash -lc "lsof -nP -p ${pid} -iTCP | head -n 120"
    done
  fi
fi

section "Localhost peer ports used by Windsurf"
if [[ -n "${PIDS}" ]]; then
  peer_ports="$(
    for pid in ${PIDS}; do
      lsof -nP -p "${pid}" -iTCP 2>/dev/null | awk '{print $9}'
    done | grep -E '127\.0\.0\.1:[0-9]+' | sed -E 's/.*127\.0\.0\.1:([0-9]+).*/\1/' | sort -n | uniq
  )"

  if [[ -z "${peer_ports}" ]]; then
    echo "No 127.0.0.1 peer ports found in Windsurf TCP connections."
  else
    # Only show peer ports that actually have a listener, and cap output to avoid truncation.
    shown=0
    for p in ${peer_ports}; do
      listener="$(lsof -nP -iTCP:"${p}" -sTCP:LISTEN 2>/dev/null | head -n 5 || true)"
      if [[ -n "${listener}" ]]; then
        run "Who listens on 127.0.0.1:${p}" lsof -nP -iTCP:"${p}" -sTCP:LISTEN
        shown=$((shown + 1))
        if [[ "${shown}" -ge 40 ]]; then
          echo "(peer listener list truncated at 40 ports)" | redact
          break
        fi
      fi
    done
    if [[ "${shown}" -eq 0 ]]; then
      echo "Peer ports exist but none had LISTENers (may be ephemeral client ports)." | redact
    fi
  fi
fi

section "Config scan (non-sensitive patterns)"
SUPPORT_DIR="$HOME/Library/Application Support/Windsurf"
if [[ -d "${SUPPORT_DIR}" ]]; then
  echo "Scanning: ${SUPPORT_DIR}"
  echo "(Only small text-like files; output redacted)"

  section "Key config files (targeted grep)"
  KEY_FILES=(
    "${SUPPORT_DIR}/User/settings.json"
    "${SUPPORT_DIR}/User/keybindings.json"
  )
  for f in "${KEY_FILES[@]}"; do
    if [[ -f "${f}" ]]; then
      echo "-- ${f}"
      grep -nH -I -E "(localhost|127\\.0\\.0\\.1|0\\.0\\.0\\.0|ws://|wss://|proxy|endpoint|baseUrl|base_url|api_url|openai|anthropic|claude)" "${f}" 2>/dev/null | head -n 120 | redact || true
    fi
  done

  tmp_list="$(mktemp -t windsurf_probe_files.XXXXXX)"
  find "${SUPPORT_DIR}" \
    \( \
      -path "${SUPPORT_DIR}/clp" -o \
      -path "${SUPPORT_DIR}/clp/*" -o \
      -path "${SUPPORT_DIR}/clp/*/*" -o \
      -path "${SUPPORT_DIR}/User/workspaceStorage" -o \
      -path "${SUPPORT_DIR}/User/workspaceStorage/*" -o \
      -path "${SUPPORT_DIR}/User/History" -o \
      -path "${SUPPORT_DIR}/User/History/*" -o \
      -path "${SUPPORT_DIR}/logs" -o \
      -path "${SUPPORT_DIR}/logs/*" -o \
      -path "${SUPPORT_DIR}/User/workspaceStorage/*/*/.metadata" -o \
      -path "${SUPPORT_DIR}/User/workspaceStorage/*/*/.metadata/*" -o \
      -path "${SUPPORT_DIR}/User/workspaceStorage/*/*/.metadata/*/*" \
    \) -prune -o \
    -type f -size -2000k \( \
      -name "*.json" -o -name "*.toml" -o -name "*.yaml" -o -name "*.yml" -o -name "*.ini" -o -name "*.txt" -o -name "*.log" -o -name "*.js" -o -name "*.ts" \
    \) -print 2>/dev/null | head -n 2000 > "${tmp_list}" || true

  file_count="$(wc -l < "${tmp_list}" | tr -d ' ')"
  if [[ "${file_count}" == "0" ]]; then
    echo "No candidate files found under support dir."
    rm -f "${tmp_list}" || true
  else
    printf 'candidate_files %s\n' "${file_count}"

    cat "${tmp_list}" | xargs -I{} grep -nH -I -E \
      "(localhost|127\\.0\\.0\\.1|0\\.0\\.0\\.0|ws://|wss://|proxy|endpoint|baseUrl|base_url|api_url|openai|anthropic|claude)" \
      "{}" 2>/dev/null | head -n 120 | redact || true

    rm -f "${tmp_list}" || true
  fi
else
  echo "Support dir not found: ${SUPPORT_DIR}"
fi

section "App bundle hints (lightweight)"
APP_DIR="/Applications/Windsurf.app/Contents"
if [[ -d "${APP_DIR}" ]]; then
  echo "Listing a few likely resources under: ${APP_DIR}"
  find "${APP_DIR}" -maxdepth 4 -type f \( -name "*.json" -o -name "*.plist" -o -name "*.asar" \) 2>/dev/null | head -n 80 | redact
else
  echo "App dir not found: ${APP_DIR}"
fi

section "Next"
echo "BRIEF=${BRIEF} (set BRIEF=0 for verbose output)" | redact
echo "1) Ensure Windsurf is open and you have an active chat session." 
echo "2) Focus on: LISTEN ports, localhost peer ports, and settings.json hits." 
