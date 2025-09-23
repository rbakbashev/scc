int fib(int n)
{
	int x0 = 0;
	int x1 = 1;
	int sum;

	if (n <= 1)
		return n;

	while (n >= 2) {
		sum = x0 + x1;
		x0 = x1;
		x1 = sum;

		n -= 1;
	}

	return x1;
}

int main()
{
	return fib(10);
}
