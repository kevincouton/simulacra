#!/usr/bin/env python3
"""Verify Simulacra discovery, messaging, and retrieval with the official A2A SDK."""

import argparse
import asyncio
import json

import httpx
from a2a.client import A2ACardResolver, ClientConfig, create_client
from a2a.helpers import new_text_message
from a2a.types import GetTaskRequest, Role, SendMessageRequest


async def check_version(base_url: str, card_path: str, text: str) -> str:
    """Run one non-streaming A2A exchange and return the created task ID."""
    async with httpx.AsyncClient(timeout=30) as http:
        resolver = A2ACardResolver(http, base_url, agent_card_path=card_path)
        card = await resolver.get_agent_card()
        assert card.name == "Simulacra Research Agent"
        assert not card.capabilities.streaming
        assert card.supported_interfaces

        # Resolve through the requested base URL so isolated tests never
        # accidentally send traffic to the URL advertised by a public card.
        for interface in card.supported_interfaces:
            suffix = "/a2a/v1" if interface.protocol_version == "1.0" else "/a2a/"
            interface.url = base_url.rstrip("/") + suffix

        config = ClientConfig(streaming=False, httpx_client=http)
        client = await create_client(agent=card, client_config=config)
        try:
            request = SendMessageRequest(
                message=new_text_message(text=text, role=Role.ROLE_USER)
            )
            chunks = [chunk async for chunk in client.send_message(request)]
            assert len(chunks) == 1, f"expected one response, got {len(chunks)}"
            assert chunks[0].WhichOneof("payload") == "task"
            task = chunks[0].task
            assert task.id.startswith("simulacra-task-")
            assert len(task.artifacts) == 1
            assert len(task.history) == 2

            fetched = await client.get_task(GetTaskRequest(id=task.id))
            assert fetched.id == task.id
            assert fetched.status.state == task.status.state
            assert len(fetched.artifacts) == len(task.artifacts)
            print(
                json.dumps(
                    {
                        "card": card_path,
                        "task_id": task.id,
                        "state": task.status.state,
                        "artifacts": len(task.artifacts),
                        "history": len(task.history),
                        "retrieval": "ok",
                    }
                )
            )
            return task.id
        finally:
            await client.close()


async def main() -> None:
    """Check A2A 1.0 and the 0.3 compatibility mode."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-url", default="https://simulation.lucanian.app")
    parser.add_argument("--text", default="run gossip seed 42")
    args = parser.parse_args()

    for card_path in ("/.well-known/agent-card.json", "/.well-known/agent.json"):
        await check_version(args.base_url, card_path, args.text)


if __name__ == "__main__":
    asyncio.run(main())
