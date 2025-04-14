import React from 'react';
import PollCard from '@/components/Cards/PollCard';
import { PollResult } from '@/model/poll.model';
import { useVoteManagementContext } from '@/context/voteManagement';
import { Link } from 'react-router-dom';

type PastPollSectionProps = {
  customLabel?: string;
  useFullHeight?: boolean;
  limit?: number;
};

const PastPollSection: React.FC<PastPollSectionProps> = ({ customLabel = 'Past polls', useFullHeight = true, limit }) => {
  const { pastPolls } = useVoteManagementContext();
  const pollsToShow = limit ? pastPolls.slice(0, limit) : pastPolls;

  return (
    <div className={`flex ${useFullHeight ? 'min-h-screen' : ''} w-screen flex-col items-center justify-center space-y-12 px-6 py-32`}>
      <h1 className='text-h1 font-bold text-slate-600'>{customLabel}</h1>
      <div className='flex w-full flex-wrap justify-center gap-16 md:gap-8'>
        {pollsToShow.map((poll: PollResult) => (
          <PollCard key={poll.roundId} {...poll} />
        ))}
      </div>
      <Link to={'/historic'}>
        <button className='button-outlined button-max'>view all polls</button>
      </Link>
    </div>
  );
};

export default PastPollSection;