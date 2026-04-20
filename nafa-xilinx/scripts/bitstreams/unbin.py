def main(input: str, output: str):
    output_data = bytearray()
    with open(input, "r") as f:
        for line in f:
            line = line.strip()
            if line[0] != "0" and line[0] != "1":
                continue
            assert len(line) % 8 == 0, f"line len {len(line)}"
            output_data.extend(
                int(line[idx : idx + 8], base=2) for idx in range(0, len(line), 8)
            )
    with open(output, "w") as f:
        _ = f.buffer.write(bytes(output_data))


if __name__ == "__main__":
    import sys

    main(*sys.argv[1:])
