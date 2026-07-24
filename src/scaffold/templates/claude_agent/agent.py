from __future__ import annotations

from claude_agent_sdk import query, ClaudeAgentOptions

options = ClaudeAgentOptions(model="{{model_name}}")


async def run(message: str) -> str:
    chunks: list[str] = []
    async for event in query(prompt=message, options=options):
        if hasattr(event, "result") and event.result:
            chunks.append(event.result)
    return "".join(chunks)
