from __future__ import annotations

from datetime import datetime, timezone

from strands import Agent, tool


@tool
def current_time() -> str:
    """Return the current UTC time in ISO 8601 format."""
    return datetime.now(timezone.utc).isoformat()


agent = Agent(
    name="{{agent_name}}",
    system_prompt="You are a helpful production assistant for {{agent_name_kebab}}.",
    tools=[current_time],
)


async def run(message: str) -> str:
    result = await agent.invoke_async(message)
    return str(result)
