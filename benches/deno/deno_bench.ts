import { Cron } from "https://deno.land/x/croner@7.0.5-dev.0/dist/croner.js";

function workload() {
  new Cron("15 15 15 L 3 *").nextRuns(100);
}

// url_bench.ts
Deno.bench("parse take 100", () => {
  workload();
});
