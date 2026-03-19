import hashlib
import time
from typing import Tuple


def generate_hashcash(resource: str, difficulty: int = 20) -> Tuple[str, int]:
    """
    Generate a hashcash proof-of-work stamp.
    
    Args:
        resource: The resource being protected (e.g., email address)
        difficulty: Number of leading zero bits required (higher = harder)
    
    Returns:
        Tuple of (stamp, counter) where stamp is the hashcash string
    """
    version = "1"
    timestamp = int(time.time())
    counter = 0
    
    while True:
        stamp = f"{version}:{difficulty}:{timestamp}:{resource}:{counter}"
        hash_value = hashlib.sha256(stamp.encode()).hexdigest()
        
        # Count leading zero bits
        leading_zeros = count_leading_zero_bits(hash_value)
        
        if leading_zeros >= difficulty:
            return stamp, counter
        
        counter += 1


def count_leading_zero_bits(hex_hash: str) -> int:
    """
    Count the number of leading zero bits in a hexadecimal hash.
    
    Each hex digit represents 4 bits. We count:
    - Full zero hex digits: 4 bits each
    - Partial zero bits in the first non-zero hex digit
    """
    bits = 0
    
    for char in hex_hash:
        if char == '0':
            bits += 4
        else:
            # Count leading zero bits in this hex digit
            value = int(char, 16)
            if value == 0:
                bits += 4
            else:
                # Count leading zeros in binary representation
                binary = bin(value)[2:].zfill(4)
                bits += len(binary) - len(binary.lstrip('0'))
                break
    
    return bits


def verify_hashcash(stamp: str) -> bool:
    """
    Verify that a hashcash stamp is valid.
    
    Args:
        stamp: The hashcash stamp string
    
    Returns:
        True if valid, False otherwise
    """
    try:
        parts = stamp.split(':')
        if len(parts) != 5:
            return False
        
        version, claimed_difficulty, timestamp, resource, counter = parts
        
        hash_value = hashlib.sha256(stamp.encode()).hexdigest()
        leading_zeros = count_leading_zero_bits(hash_value)
        
        return leading_zeros >= int(claimed_difficulty)
    except (ValueError, IndexError):
        return False


def demonstrate():
    """Demonstrate the hashcash algorithm with examples."""
    print("Hashcash Algorithm Demonstration")
    print("=" * 50)
    
    resource = "user@example.com"
    
    print(f"\nGenerating hashcash for: {resource}")
    print("Difficulty: 16 bits (moderate difficulty)")
    
    start_time = time.time()
    stamp, counter = generate_hashcash(resource, difficulty=16)
    elapsed = time.time() - start_time
    
    print(f"\nFound solution after {counter} attempts in {elapsed:.2f} seconds")
    print(f"Stamp: {stamp}")
    
    hash_value = hashlib.sha256(stamp.encode()).hexdigest()
    print(f"Hash: {hash_value}")
    print(f"Leading zero bits: {count_leading_zero_bits(hash_value)}")
    
    print(f"\nVerifying stamp...")
    is_valid = verify_hashcash(stamp)
    print(f"Valid: {is_valid}")
    
    print("\n" + "=" * 50)
    print("Trying higher difficulty (20 bits)...")
    
    start_time = time.time()
    stamp2, counter2 = generate_hashcash(resource, difficulty=20)
    elapsed2 = time.time() - start_time
    
    print(f"Found solution after {counter2} attempts in {elapsed2:.2f} seconds")
    hash_value2 = hashlib.sha256(stamp2.encode()).hexdigest()
    print(f"Hash: {hash_value2}")
    print(f"Leading zero bits: {count_leading_zero_bits(hash_value2)}")


if __name__ == "__main__":
    demonstrate()

