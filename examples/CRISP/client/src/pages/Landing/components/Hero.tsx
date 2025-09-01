// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React from 'react'
import Logo from '@/assets/icons/logo.svg'
import CircularTiles from '@/components/CircularTiles'
import { Link } from 'react-router-dom'
import { Keyhole, ListMagnifyingGlass, ShieldCheck } from '@phosphor-icons/react'

const HeroSection: React.FC = () => {
  return (
    <div className='relative flex w-full flex-1 items-center justify-center px-6'>
      <div className='absolute bottom-0 right-0 grid w-full grid-cols-2 gap-2 max-md:opacity-50 md:w-[70vh]'>
        <CircularTiles count={4} />
      </div>
      <div className='relative mx-auto w-full max-w-screen-md space-y-12'>
        <div className='space-y-4'>
          <h3 className='font-normal leading-none text-zinc-400 sm:text-xl md:text-3xl'>Introducing</h3>
          <img src={Logo} alt='CRISP Logo' className='sm:h-10 md:h-20' />
          <h4 className='w-full leading-none text-slate-800/50 sm:text-xs md:text-base'>
            Coercion-Resistant Impartial Selection Protocol
          </h4>
        </div>
        <ul className='space-y-3'>
          <li className='flex items-start space-x-2 md:items-center'>
            <Keyhole className='text-lime-600/80' size={32} />
            <div className='text-zinc-400 sm:text-sm md:text-lg'>
              <span className='mr-1 font-bold text-lime-600/80'>Private.</span>
              Voter privacy through advanced encryption.
            </div>
          </li>
          <li className='flex items-start space-x-2 md:items-center'>
            <ListMagnifyingGlass className='text-lime-600/80' size={32} />
            <div className='text-zinc-400 sm:text-sm md:text-lg'>
              <span className='mr-1 font-bold text-lime-600/80'>Reliable.</span>
              Verifiable results while preserving confidentiality.
            </div>
          </li>
          <li className='flex items-start space-x-2 md:items-center'>
            <ShieldCheck className='text-lime-600/80' size={32} />
            <div className='text-zinc-400 sm:text-sm md:text-lg'>
              <span className='mr-1 font-bold text-lime-600/80'>Equitable.</span>
              Robust safeguards against coercion and tampering.
            </div>
          </li>
        </ul>
        <div className='space-y-4'>
          <div className='flex flex-wrap items-center text-sm md:space-x-2'>
            <div className='text-slate-400'>This is a simple demonstration of CRISP technology.</div>
            <Link
              target='_blank'
              to='https://docs.enclave.gg'
              className='inline-flex cursor-pointer items-center space-x-1 text-lime-600 duration-300 ease-in-out hover:underline hover:opacity-70'
            >
              <div>Learn more.</div>
            </Link>
          </div>
          <Link to='/current' className='inline-flex'>
            <button className='button-primary'>Try Demo</button>
          </Link>
        </div>
      </div>
    </div>
  )
}

export default HeroSection
