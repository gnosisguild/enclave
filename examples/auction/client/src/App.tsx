import { useCallback, useEffect, useState } from "react";
import CreateAuction from "./components/CreateAuction";
import SubmitBid from "./components/SubmitBid";
import AuctionResult from "./components/AuctionResult";
import { getAuction, AuctionInfo } from "./hooks/useAuction";

export default function App() {
  const [auctionId, setAuctionId] = useState<number | null>(null);
  const [info, setInfo] = useState<AuctionInfo | null>(null);

  const refresh = useCallback(async () => {
    if (auctionId === null) return;
    try {
      setInfo(await getAuction(auctionId));
    } catch {
      /* ignore */
    }
  }, [auctionId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <div className="container">
      <h1>Sealed-Bid Auction</h1>
      <p className="subtitle">Encrypted bids compared homomorphically with BFV FHE</p>

      {auctionId === null ? (
        <CreateAuction onCreated={(id) => setAuctionId(id)} />
      ) : (
        <>
          <SubmitBid auctionId={auctionId} onBidSubmitted={refresh} />
          <AuctionResult
            auctionId={auctionId}
            numBids={info?.num_bids ?? 0}
          />
          {info?.state === "complete" && (
            <button
              className="new-auction"
              onClick={() => {
                setAuctionId(null);
                setInfo(null);
              }}
            >
              New Auction
            </button>
          )}
        </>
      )}
    </div>
  );
}
