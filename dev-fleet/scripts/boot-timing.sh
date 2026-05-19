#!/usr/bin/env bash
set -euo pipefail

# Measures sandbox boot times with phase breakdowns.
# Requires: fleet running, jq, curl, python3 (for ms timestamps)

CONTROLLER="${CONTROLLER:-http://localhost:8080}"
PROXY="${PROXY:-http://localhost:9090}"
CLUSTER="${CLUSTER:-localhost:9090}"
IMAGE="${IMAGE:-sandbox-runtime:node24-latest}"
POOL="${POOL:-trusted}"
RUNS="${1:-5}"

now_ms() { python3 -c 'import time; print(int(time.time()*1000))'; }
delta() { echo $(( $2 - $1 )); }

echo "=== Sandbox Boot Timing ==="
echo "Image:   $IMAGE"
echo "Pool:    $POOL"
echo "Runs:    $RUNS"
echo ""

declare -a total_times spawn_times ready_times exec_times

for i in $(seq 1 "$RUNS"); do
  KEY="boot-timing-$(date +%s)-$i"
  BACKEND_ID="bt-$(date +%s)-$i"

  # --- Phase 1: Spawn request ---
  t_start=$(now_ms)

  RESP=$(curl -s -w '\n%{http_code}' -X POST "$CONTROLLER/ctrl/connect" \
    -H 'Content-Type: application/json' \
    -d "{
      \"key\": {\"name\": \"$KEY\", \"namespace\": \"\", \"tag\": \"\"},
      \"spawn_config\": {
        \"id\": \"$BACKEND_ID\",
        \"cluster\": \"$CLUSTER\",
        \"pool\": \"$POOL\",
        \"executable\": {
          \"image\": \"$IMAGE\",
          \"env\": {}
        },
        \"lifetime_limit_seconds\": 120,
        \"max_idle_seconds\": 30
      }
    }")

  HTTP_CODE=$(echo "$RESP" | tail -1)
  BODY=$(echo "$RESP" | sed '$d')

  if [ "$HTTP_CODE" != "200" ]; then
    echo "  Run $i: FAILED spawn (HTTP $HTTP_CODE): $BODY"
    continue
  fi

  TOKEN=$(echo "$BODY" | jq -r '.token')
  SPAWNED=$(echo "$BODY" | jq -r '.spawned')
  DRONE=$(echo "$BODY" | jq -r '.drone // "unknown"')

  t_spawn=$(now_ms)
  spawn_ms=$(delta "$t_start" "$t_spawn")

  # --- Phase 2: Wait for ready ---
  READY=false
  for attempt in $(seq 1 200); do
    STATUS=$(curl -s "$CONTROLLER/pub/b/$BACKEND_ID/status" 2>/dev/null || echo '{}')
    STATE=$(echo "$STATUS" | jq -r '.status // "unknown"')
    if [ "$STATE" = "ready" ]; then
      READY=true
      break
    elif [ "$STATE" = "terminated" ] || [ "$STATE" = "hard_terminating" ]; then
      echo "  Run $i: sandbox terminated unexpectedly (status=$STATE)"
      break
    fi
    sleep 0.1
  done

  t_ready=$(now_ms)
  ready_ms=$(delta "$t_spawn" "$t_ready")
  total_boot_ms=$(delta "$t_start" "$t_ready")

  if [ "$READY" != "true" ]; then
    echo "  Run $i: FAILED to reach ready (last status: $STATE) after ${total_boot_ms}ms"
    # cleanup
    curl -s -X POST "$CONTROLLER/ctrl/b/$BACKEND_ID/hard-terminate" > /dev/null 2>&1 || true
    continue
  fi

  # --- Phase 3: First exec (measure agentd responsiveness) ---
  EXEC_RESP=$(curl -s -X POST "$PROXY/$TOKEN/v1/exec" \
    -H "Host: $CLUSTER" \
    -H 'Content-Type: application/json' \
    -d '{"cmd": "/bin/sh", "args": ["-c", "echo ok"], "timeout_ms": 5000}')

  t_exec=$(now_ms)
  exec_ms=$(delta "$t_ready" "$t_exec")
  exec_exit=$(echo "$EXEC_RESP" | jq -r '.exit_code // "err"')
  agentd_duration=$(echo "$EXEC_RESP" | jq -r '.duration_ms // "?"')

  # --- Phase 4: Detailed status for breakdown ---
  DETAIL=$(curl -s "$CONTROLLER/pub/b/$BACKEND_ID/status" 2>/dev/null || echo '{}')

  # --- Cleanup ---
  curl -s -X POST "$CONTROLLER/ctrl/b/$BACKEND_ID/soft-terminate" > /dev/null 2>&1 || true

  # --- Report ---
  printf "  Run %d/%d [%s]:  spawn=%dms  ready=%dms  exec=%dms  total=%dms  (agentd_exec=%sms)\n" \
    "$i" "$RUNS" "$DRONE" "$spawn_ms" "$ready_ms" "$exec_ms" "$total_boot_ms" "$agentd_duration"

  total_times+=("$total_boot_ms")
  spawn_times+=("$spawn_ms")
  ready_times+=("$ready_ms")
  exec_times+=("$exec_ms")

  # wait for cleanup before next run
  sleep 1
done

echo ""

if [ ${#total_times[@]} -eq 0 ]; then
  echo "No successful runs."
  exit 1
fi

# --- Summary statistics ---
sorted_total=($(printf '%s\n' "${total_times[@]}" | sort -n))
sorted_spawn=($(printf '%s\n' "${spawn_times[@]}" | sort -n))
sorted_ready=($(printf '%s\n' "${ready_times[@]}" | sort -n))
sorted_exec=($(printf '%s\n' "${exec_times[@]}" | sort -n))
n=${#sorted_total[@]}

sum() { local s=0; for v in "$@"; do s=$((s+v)); done; echo $s; }
p95_idx=$(( (n * 95 + 99) / 100 - 1 ))
[ "$p95_idx" -ge "$n" ] && p95_idx=$((n-1))
[ "$p95_idx" -lt 0 ] && p95_idx=0

echo "=== Summary ($n successful runs) ==="
echo ""
printf "  %-14s %8s %8s %8s %8s\n" "Phase" "min" "median" "p95" "max"
printf "  %-14s %8s %8s %8s %8s\n" "-----" "---" "------" "---" "---"
printf "  %-14s %5dms  %5dms  %5dms  %5dms\n" "spawn" "${sorted_spawn[0]}" "${sorted_spawn[$((n/2))]}" "${sorted_spawn[$p95_idx]}" "${sorted_spawn[$((n-1))]}"
printf "  %-14s %5dms  %5dms  %5dms  %5dms\n" "ready" "${sorted_ready[0]}" "${sorted_ready[$((n/2))]}" "${sorted_ready[$p95_idx]}" "${sorted_ready[$((n-1))]}"
printf "  %-14s %5dms  %5dms  %5dms  %5dms\n" "first_exec" "${sorted_exec[0]}" "${sorted_exec[$((n/2))]}" "${sorted_exec[$p95_idx]}" "${sorted_exec[$((n-1))]}"
printf "  %-14s %5dms  %5dms  %5dms  %5dms\n" "TOTAL" "${sorted_total[0]}" "${sorted_total[$((n/2))]}" "${sorted_total[$p95_idx]}" "${sorted_total[$((n-1))]}"
echo ""
echo "  spawn    = controller HTTP round-trip (POST /ctrl/connect)"
echo "  ready    = time from spawn response until backend reports ready"
echo "  first_exec = proxy round-trip for first exec after ready"
echo "  TOTAL    = spawn + ready (end-to-end cold start)"
