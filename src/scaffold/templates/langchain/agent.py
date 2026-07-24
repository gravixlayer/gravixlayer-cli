from __future__ import annotations

from datetime import datetime, timezone

from langchain.agents import create_agent

def current_time() -> str:
    """Return the current UTC time in ISO 8601 format."""
    return datetime.now(timezone.utc).isoformat()


agent = create_agent(
    model="{{langchain_model_id}}",
    tools=[current_time],
    system_prompt="You are a helpful production assistant for {{agent_name_kebab}}.",
)


async def run(message: str) -> str:
    result = await agent.ainvoke(
        {"messages": [{"role": "user", "content": message}]}
    )
    return result["messages"][-1].content
