import type { ProviderId } from "@ai-usage-dashboard/core"
import claudeLogo from "../assets/logos/claude.svg"
import codexLogo from "../assets/logos/codex.svg"
import copilotLogo from "../assets/logos/copilot.svg"
import kimiLogo from "../assets/logos/kimi.svg"
import openrouterLogo from "../assets/logos/openrouter.svg"
import alibabaTokenPlanLogo from "../assets/logos/alibaba-token-plan.svg"
import openCodeGoLogo from "../assets/logos/opencode-go.svg"
import grokLogo from "../assets/logos/grok.svg"

export const providerLogo: Record<ProviderId, string> = {
  claude: claudeLogo,
  codex: codexLogo,
  copilot: copilotLogo,
  openrouter: openrouterLogo,
  "alibaba-token-plan": alibabaTokenPlanLogo,
  "opencode-go": openCodeGoLogo,
  grok: grokLogo,
  kimi: kimiLogo
}
