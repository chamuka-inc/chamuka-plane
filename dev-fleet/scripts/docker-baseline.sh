#!/usr/bin/env bash
set -euo pipefail

# Measures raw Docker container boot time — no controller, no drone.
# Direct docker run + poll agentd /health until it responds.

IMAGE="${IMAGE:-sandbox-runtime:node24-latest}"
RUNS="${1:-5}"

now_ms() { python3 -c 'import time; print(int(time.time()*1000))'; }

echo "=== Docker Baseline Boot Timing ==="
echo "Image: $IMAGE"
echo "Runs:  $RUNS"
echo ""

declare -a create_times start_times health_times total_times

for i in $(seq 1 "$RUNS"); do
  NAME="baseline-test-$i-$(date +%s)"

  # --- Phase 1: docker create ---
  t0=$(now_ms)
  CID=$(docker create --name "$NAME" -p 0:8080 "$IMAGE" 2>&1)
  t_create=$(now_ms)
  create_ms=$(( t_create - t0 ))

  # --- Phase 2: docker start ---
  docker start "$NAME" > /dev/null 2>&1
  t_start=$(now_ms)
  start_ms=$(( t_start - t_create ))

  # Get the mapped port
  PORT=$(docker port "$NAME" 8080 2>/dev/null | head -1 | cut -d: -f2)

  # --- Phase 3: poll /health until 200 ---
  HEALTHY=false
  for attempt in $(seq 1 200); do
    if curl -sf -o /dev/null --max-time 1 "http://localhost:$PORT/health" 2>/dev/null; then
      HEALTHY=true
      break
    fi
    sleep 0.05
  done
  t_healthy=$(now_ms)
  health_ms=$(( t_healthy - t_start ))
  total_ms=$(( t_healthy - t0 ))

  if [ "$HEALTHY" = "true" ]; then
    # bonus: measure exec latency
    EXEC_RESP=$(curl -s -X POST "http://localhost:$PORT/v1/exec" \
      -H 'Content-Type: application/json' \
      -d '{"cmd": "/bin/sh", "args": ["-c", "echo ok"], "timeout_ms": 5000}' 2>/dev/null || echo '{}')
    exec_dur=$(echo "$EXEC_RESP" | jq -r '.duration_ms // "?"')

    printf "  Run %d/%d:  create=%dms  start=%dms  health=%dms  total=%dms  (exec=%sms)\n" \
      "$i" "$RUNS" "$create_ms" "$start_ms" "$health_ms" "$total_ms" "$exec_dur"

    create_times+=("$create_ms")
    start_times+=("$start_ms")
    health_times+=("$health_ms")
    total_times+=("$total_ms")
  else
    printf "  Run %d/%d:  FAILED to become healthy after %dms\n" "$i" "$RUNS" "$total_ms"
  fi

  # cleanup
  docker rm -f "$NAME" > /dev/null 2>&1 || true
  sleep 0.5
done

echo ""

if [ ${#total_times[@]} -eq 0 ]; then
  echo "No successful runs."
  exit 1
fi

sorted_create=($(printf '%s\n' "${create_times[@]}" | sort -n))
sorted_start=($(printf '%s\n' "${start_times[@]}" | sort -n))
sorted_health=($(printf '%s\n' "${health_times[@]}" | sort -n))
sorted_total=($(printf '%s\n' "${total_times[@]}" | sort -n))
n=${#sorted_total[@]}
p95_idx=$(( (n * 95 + 99) / 100 - 1 ))
[ "$p95_idx" -ge "$n" ] && p95_idx=$((n-1))
[ "$p95_idx" -lt 0 ] && p95_idx=0

echo "=== Summary ($n successful runs) ==="
echo ""
printf "  %-14s %8s %8s %8s %8s\n" "Phase" "min" "median" "p95" "max"
printf "  %-14s %8s %8s %8s %8s\n" "-----" "---" "------" "---" "---"
printf "  %-14s %5dms  %5dms  %5dms  %5dms\n" "docker_create" "${sorted_create[0]}" "${sorted_create[$((n/2))]}" "${sorted_create[$p95_idx]}" "${sorted_create[$((n-1))]}"
printf "  %-14s %5dms  %5dms  %5dms  %5dms\n" "docker_start" "${sorted_start[0]}" "${sorted_start[$((n/2))]}" "${sorted_start[$p95_idx]}" "${sorted_start[$((n-1))]}"
printf "  %-14s %5dms  %5dms  %5dms  %5dms\n" "health_ready" "${sorted_health[0]}" "${sorted_health[$((n/2))]}" "${sorted_health[$p95_idx]}" "${sorted_health[$((n-1))]}"
printf "  %-14s %5dms  %5dms  %5dms  %5dms\n" "TOTAL" "${sorted_total[0]}" "${sorted_total[$((n/2))]}" "${sorted_total[$p95_idx]}" "${sorted_total[$((n-1))]}"
echo ""
echo "  docker_create  = docker create (overlay setup, cgroup init)"
echo "  docker_start   = docker start (process launch)"
echo "  health_ready   = time until agentd /health responds 200"
echo "  TOTAL          = end-to-end without controller/drone overhead"
