// The Timer tab: a pomodoro and a stopwatch, as the original has.
//
// Both count from a wall-clock start rather than by accumulating ticks. An
// interval that fires late — which it will, in a webview the compositor is
// free to throttle — must not make the timer run slow.

import { useEffect, useRef, useState } from "react";
import { Button, Segmented, Symbol } from "../../widgets";
import { tr } from "../../i18n";

/** The original's defaults, in minutes. */
const WORK_MINUTES = 25;
const BREAK_MINUTES = 5;

export function formatDuration(milliseconds: number): string {
  const total = Math.max(0, Math.floor(milliseconds / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  const pad = (value: number) => String(value).padStart(2, "0");
  return hours > 0
    ? `${hours}:${pad(minutes)}:${pad(seconds)}`
    : `${pad(minutes)}:${pad(seconds)}`;
}

/** Ticks while `running`, reporting elapsed wall-clock milliseconds. */
function useElapsed(running: boolean, resetKey: number): number {
  const [elapsed, setElapsed] = useState(0);
  const startedAt = useRef<number | null>(null);
  const carried = useRef(0);

  useEffect(() => {
    startedAt.current = null;
    carried.current = 0;
    setElapsed(0);
  }, [resetKey]);

  useEffect(() => {
    if (!running) {
      // Pausing banks what has run so far; resuming starts a new segment.
      if (startedAt.current !== null) {
        carried.current += Date.now() - startedAt.current;
        startedAt.current = null;
        setElapsed(carried.current);
      }
      return;
    }

    startedAt.current = Date.now();
    const tick = () => {
      if (startedAt.current === null) return;
      setElapsed(carried.current + (Date.now() - startedAt.current));
    };
    tick();

    // 200ms rather than 1000: the display shows seconds, and a 1s interval
    // that drifts makes the last digit visibly stutter.
    const timer = window.setInterval(tick, 200);
    return () => window.clearInterval(timer);
  }, [running, resetKey]);

  return elapsed;
}

export function Timer() {
  const [mode, setMode] = useState<"pomodoro" | "stopwatch">("pomodoro");
  return (
    <div className="bw-timer">
      <Segmented
        value={mode}
        options={[
          { value: "pomodoro", label: tr("Pomodoro") },
          { value: "stopwatch", label: tr("Stopwatch") },
        ]}
        onChange={setMode}
      />
      {mode === "pomodoro" ? <Pomodoro /> : <Stopwatch />}
    </div>
  );
}

function Pomodoro() {
  const [running, setRunning] = useState(false);
  const [resting, setResting] = useState(false);
  const [round, setRound] = useState(0);
  const elapsed = useElapsed(running, round);

  const target = (resting ? BREAK_MINUTES : WORK_MINUTES) * 60_000;
  const remaining = Math.max(0, target - elapsed);
  const done = remaining === 0;

  useEffect(() => {
    if (!done || !running) return;
    // Rolling straight into the next phase is what makes it a pomodoro rather
    // than a countdown; it stops so the user notices.
    setRunning(false);
    setResting((value) => !value);
    setRound((value) => value + 1);
  }, [done, running]);

  const fraction = target === 0 ? 0 : 1 - remaining / target;

  return (
    <div className="bw-timer-face">
      <div
        className="bw-timer-ring"
        style={{ "--fraction": fraction } as never}
      >
        <span className="bw-timer-value">{formatDuration(remaining)}</span>
      </div>
      <span className="bw-timer-phase">
        {resting ? tr("Break") : tr("Focus")}
      </span>
      <div className="bw-timer-buttons">
        <Button variant="filled" onClick={() => setRunning((value) => !value)}>
          <Symbol name={running ? "pause" : "play_arrow"} size={18} />
          {running ? tr("Pause") : tr("Start")}
        </Button>
        <Button
          variant="text"
          onClick={() => {
            setRunning(false);
            setResting(false);
            setRound((value) => value + 1);
          }}
        >
          {tr("Reset")}
        </Button>
      </div>
    </div>
  );
}

function Stopwatch() {
  const [running, setRunning] = useState(false);
  const [resetKey, setResetKey] = useState(0);
  const [laps, setLaps] = useState<number[]>([]);
  const elapsed = useElapsed(running, resetKey);

  return (
    <div className="bw-timer-face">
      <span className="bw-timer-value bw-timer-value-large">
        {formatDuration(elapsed)}
      </span>
      <div className="bw-timer-buttons">
        <Button variant="filled" onClick={() => setRunning((value) => !value)}>
          <Symbol name={running ? "pause" : "play_arrow"} size={18} />
          {running ? tr("Pause") : tr("Start")}
        </Button>
        <Button
          variant="tonal"
          disabled={!running}
          onClick={() => setLaps((current) => [elapsed, ...current])}
        >
          {tr("Lap")}
        </Button>
        <Button
          variant="text"
          onClick={() => {
            setRunning(false);
            setLaps([]);
            setResetKey((value) => value + 1);
          }}
        >
          {tr("Reset")}
        </Button>
      </div>

      {laps.length > 0 ? (
        <ol className="bw-timer-laps">
          {laps.map((lap, index) => (
            <li key={laps.length - index}>
              <span>{laps.length - index}</span>
              <span>{formatDuration(lap)}</span>
            </li>
          ))}
        </ol>
      ) : null}
    </div>
  );
}
