num = int(input())

def is_number(*, num: int) -> bool:
    if num % 2 == 0:
        return True
    return False

print(is_number(num=num))