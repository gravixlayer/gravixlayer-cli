from __future__ import annotations

from agents import Agent, Runner, function_tool
from datetime import datetime


@function_tool
def get_current_time() -> str:
    """Return the current date and time in ISO format."""
    return datetime.now().isoformat()


agent = Agent(
    name="{{agent_name}}",
    model="{{model_name}}",
    instructions="You are a helpful assistant.",
    tools=[get_current_time],
)


async def run(message: str) -> str:
    result = await Runner.run(agent, message)
    return result.final_output
