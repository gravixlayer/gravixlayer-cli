from __future__ import annotations

import os

import uvicorn
from fastapi import FastAPI
from pydantic import BaseModel

from .agent import run

app = FastAPI(title="{{agent_name}}", version="0.1.0")


class InvokeRequest(BaseModel):
    message: str


class InvokeResponse(BaseModel):
    response: str


@app.post("/invoke", response_model=InvokeResponse)
async def invoke(request: InvokeRequest) -> InvokeResponse:
    response = await run(request.message)
    return InvokeResponse(response=response)


@app.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}


if __name__ == "__main__":
    uvicorn.run(
        "app.{{agent_name}}.main:app",
        host=os.getenv("HOST", "0.0.0.0"),
        port=int(os.getenv("PORT", "{{http_port}}")),
        reload=os.getenv("RELOAD", "0") == "1",
    )
