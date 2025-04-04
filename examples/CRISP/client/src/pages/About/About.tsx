import React from 'react'
// import CircleIcon from '@/assets/icons/caretCircle.svg'
import CardContent from '@/components/Cards/CardContent'
import CircularTiles from '@/components/CircularTiles'

const About: React.FC = () => {
  return (
    <div className='relative flex w-full flex-1 items-center justify-center px-6 py-28'>
      <div className='absolute bottom-0 right-0 grid w-full grid-cols-2 gap-2 max-md:opacity-50 md:w-[70vh]'>
        <CircularTiles count={4} />
      </div>
      <div className='relative space-y-12'>
        <h1 className='text-h1 font-bold text-slate-600'>About CRISP</h1>
        <CardContent>
          <div className='space-y-4'>
            <p className='text-base font-extrabold uppercase text-slate-600/50'>what is crisp?</p>
            <div className='space-y-2'>
              <p className='leading-8 text-slate-600'>
                CRISP (Coercion-Resistant Impartial Selection Protocol) is a secure protocol for digital decision-making, leveraging fully
                homomorphic encryption (FHE) and distributed threshold cryptography (DTC) to enable verifiable secret ballots. Built with 
                Enclave, CRISP safeguards democratic systems and decision-making applications against coercion, manipulation, and other vulnerabilities.
              </p>
              {/* <div className='flex cursor-pointer items-center space-x-2'>
                <p className='text-lime-400 underline'>See what&apos;s happening under the hood</p>
                <img src={CircleIcon} className='h-[18] w-[18]' />
              </div> */}
            </div>
          </div>
          <div className='space-y-4'>
            <p className='text-base font-extrabold uppercase text-slate-600/50'>why is this important?</p>
            <p className='leading-8 text-slate-600'>
              Open ballots are known to produce suboptimal outcomes, exposing participants to bribery and coercion. CRISP mitigates these 
              risks and other vulnerabilities with secret, receipt-free ballots, fostering secure and impartial decision-making environments.
              </p>
            {/* <div className='flex cursor-pointer items-center space-x-2'>
              <p className='text-lime-400 underline'>See what&apos;s happening under the hood</p>
              <img src={CircleIcon} className='h-[18] w-[18] ' />
            </div> */}
          </div>
          <div className='space-y-4'>
            <p className='text-base font-extrabold uppercase text-slate-600/50'>Proof of Concept</p>
            <p className='leading-8 text-slate-600'>
              This application is a Proof of Concept (PoC), demonstrating the viability of Enclave as a network and CRISP as an application
              for secret ballots. Future iterations of this and other applications will be progressively more complete.
            </p>
            {/* <div className='flex cursor-pointer items-center space-x-2'>
              <p className='text-lime-400 underline'>See what&apos;s happening under the hood</p>
              <img src={CircleIcon} className='h-[18] w-[18] ' />
            </div> */}
          </div>
        </CardContent>
      </div>
    </div>
  )
}

export default About
