from __future__ import annotations

from langchain.chat_models import init_chat_model
from langgraph.graph import END, START, MessagesState, StateGraph


model = init_chat_model("{{langchain_model_id}}", temperature=0)


def call_model(state: MessagesState):
    response = model.invoke(state["messages"])
    return {"messages": [response]}


builder = StateGraph(MessagesState)
builder.add_node("agent", call_model)
builder.add_edge(START, "agent")
builder.add_edge("agent", END)

graph = builder.compile()
