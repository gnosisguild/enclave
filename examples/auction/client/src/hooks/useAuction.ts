const API = "/api";

export interface AuctionInfo {
  id: number;
  state: string;
  num_bids: number;
  public_key: string;
  result: { winner_address: string; winning_bid: number } | null;
}

export interface AuctionResult {
  winner_address: string;
  winning_bid: number;
}

export async function createAuction(): Promise<{
  id: number;
  public_key: string;
}> {
  const res = await fetch(`${API}/auction/create`, { method: "POST" });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function getAuction(id: number): Promise<AuctionInfo> {
  const res = await fetch(`${API}/auction/${id}`);
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function encryptBid(
  auctionId: number,
  bid: number,
): Promise<string> {
  const res = await fetch(`${API}/auction/${auctionId}/encrypt`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ bid }),
  });
  if (!res.ok) throw new Error(await res.text());
  const data = await res.json();
  return data.ciphertext;
}

export async function submitBid(
  auctionId: number,
  address: string,
  ciphertext: string,
): Promise<void> {
  const res = await fetch(`${API}/auction/${auctionId}/bid`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ address, ciphertext }),
  });
  if (!res.ok) throw new Error(await res.text());
}

export async function closeAuction(
  id: number,
): Promise<AuctionResult> {
  const res = await fetch(`${API}/auction/${id}/close`, { method: "POST" });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function getResult(
  id: number,
): Promise<AuctionResult> {
  const res = await fetch(`${API}/auction/${id}/result`);
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}
