import { makeHandler } from "./whakoom.mjs";
export const config = {
  path: "/api/whakoom/login",
  method: "POST",
  rateLimit: {
    action: "rate_limit",
    aggregateBy: ["domain", "ip"],
    windowSize: 60,
    windowLimit: 5,
  },
};
export default makeHandler();
