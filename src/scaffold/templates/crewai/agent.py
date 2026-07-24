from __future__ import annotations

from crewai import Agent

agent = Agent(
    role="{{agent_name}} assistant",
    goal="Answer user requests clearly and act only within the provided instructions.",
    backstory="You are a reliable production assistant for {{agent_name_kebab}}.",
    llm="{{model_name}}",
    verbose=False,
)


async def run(message: str) -> str:
    result = await agent.kickoff_async(message)
    if hasattr(result, "raw") and result.raw:
        return result.raw
    return str(result)
