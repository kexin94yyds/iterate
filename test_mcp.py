#!/usr/bin/env python3
import subprocess
import json
import sys

def test_mcp_server():
    # 启动 MCP 服务器
    process = subprocess.Popen(
        ['./target/debug/寸止'],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        cwd='/Users/apple/cunzhi/cunzhi'
    )
    
    try:
        # 发送初始化请求
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "clientInfo": {"name": "test-client", "version": "1.0.0"}
            }
        }
        
        # 发送初始化请求
        process.stdin.write(json.dumps(init_request) + '\n')
        process.stdin.flush()
        
        # 读取初始化响应
        response = process.stdout.readline()
        if response:
            print("初始化响应:", response.strip())
        
        # 发送初始化通知
        init_notification = {
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }
        
        process.stdin.write(json.dumps(init_notification) + '\n')
        process.stdin.flush()
        
        # 请求工具列表
        tools_request = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }
        
        process.stdin.write(json.dumps(tools_request) + '\n')
        process.stdin.flush()
        
        # 读取工具列表响应
        response = process.stdout.readline()
        if response:
            print("工具列表响应:", response.strip())
            tools_data = json.loads(response)
            if 'result' in tools_data and 'tools' in tools_data['result']:
                print("\n可用工具:")
                for tool in tools_data['result']['tools']:
                    print(f"- {tool['name']}: {tool.get('description', 'No description')}")
        
        # 测试寸止工具
        zhi_request = {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "zhi",
                "arguments": {
                    "message": "测试寸止工具调用",
                    "is_markdown": True
                }
            }
        }
        
        process.stdin.write(json.dumps(zhi_request) + '\n')
        process.stdin.flush()
        
        # 读取工具调用响应
        response = process.stdout.readline()
        if response:
            print("寸止工具响应:", response.strip())
        
    except Exception as e:
        print(f"错误: {e}")
    finally:
        process.terminate()
        process.wait()

if __name__ == "__main__":
    test_mcp_server()
