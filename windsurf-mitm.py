from __future__ import annotations

import json
from typing import Any

from mitmproxy import http

TARGET_HOSTS = {
    "server.self-serve.windsurf.com",
    "inference.codeium.com",
}


def _ensure_zhi(text: str) -> str:
    stripped = text.rstrip()
    if stripped.endswith("zhi"):
        return stripped
    return stripped + "\nzhi"


def _append_zhi_in_obj(obj: Any) -> bool:
    if isinstance(obj, dict):
        # OpenAI-style: {"choices": [{"message": {"content": "..."}}]}
        choices = obj.get("choices")
        if isinstance(choices, list):
            for choice in reversed(choices):
                if not isinstance(choice, dict):
                    continue
                message = choice.get("message") or choice.get("delta")
                if isinstance(message, dict):
                    content = message.get("content")
                    if isinstance(content, str):
                        message["content"] = _ensure_zhi(content)
                        return True
                text = choice.get("text")
                if isinstance(text, str):
                    choice["text"] = _ensure_zhi(text)
                    return True

        # Anthropic-style: {"content": [{"type": "text", "text": "..."}]}
        content = obj.get("content")
        if isinstance(content, str):
            obj["content"] = _ensure_zhi(content)
            return True
        if isinstance(content, list):
            for item in reversed(content):
                if not isinstance(item, dict):
                    continue
                if item.get("type") == "text" and isinstance(item.get("text"), str):
                    item["text"] = _ensure_zhi(item["text"])
                    return True

        # Legacy: {"completion": "..."}
        completion = obj.get("completion")
        if isinstance(completion, str):
            obj["completion"] = _ensure_zhi(completion)
            return True
    return False


def response(flow: http.HTTPFlow) -> None:
    host = flow.request.host
    if host not in TARGET_HOSTS:
        return

    content_type = flow.response.headers.get("content-type", "")
    if "application/grpc" in content_type:
        return

    if "application/json" in content_type:
        try:
            data = json.loads(flow.response.get_text())
        except Exception:
            return
        if _append_zhi_in_obj(data):
            flow.response.set_text(json.dumps(data))
