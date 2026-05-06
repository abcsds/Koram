export function jaccard(pairCount, totalA, totalB) {
  const denom = totalA + totalB - pairCount;
  if (denom <= 0) return 0;
  return pairCount / denom;
}
