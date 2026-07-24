from __future__ import annotations

from google.adk.agents import Agent

root_agent = Agent(
    name="{{agent_package}}",
    model="{{model_name}}",
    description="{{description}}",
    instruction="You are a helpful assistant.",
)
