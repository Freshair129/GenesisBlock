import sys
import os
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from genesisdb import GenesisClient

def main():
    # Note: Requires a running GenesisDB server on localhost:3000
    try:
        client = GenesisClient("http://localhost:3000")
        
        print("1. Adding node...")
        node = client.add_node(
            id="python-test",
            labels=["RESEARCH"],
            props={"focus": "automated-reasoning"}
        )
        print(f"Node created: {node.id}")

        print("\n2. Retrieving context (H1)...")
        ctx = client.get_context(target="python-test", tier="H1")
        print(f"Path: {ctx.reasoning_path}")
        print(f"Nodes found: {[n.id for n in ctx.nodes]}")

        print("\n3. Raw HQL query...")
        res = client.query("SEARCH Node SIMILAR TO [0.1, 0.2] K 1")
        print(f"Query Result: {res}")

    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    main()
