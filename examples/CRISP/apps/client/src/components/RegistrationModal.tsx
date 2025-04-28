import React from 'react';
import Modal from '@/components/Modal'; // Assuming you have a generic Modal component

interface RegistrationModalProps {
  isOpen: boolean;
  onClose: () => void;
  onRegister: () => void;
  isRegistering: boolean;
  explanation?: string; // Optional custom explanation
}

const DEFAULT_EXPLANATION = (
  <> 
    To vote anonymously using Semaphore, you first need to register your unique identity 
    with the polling group for this specific round. This action is required once per poll 
    and links your cryptographic identity to the group without revealing your wallet address during the vote.
  </>
);

const RegistrationModal: React.FC<RegistrationModalProps> = ({ 
  isOpen, 
  onClose, 
  onRegister, 
  isRegistering, 
  explanation 
}) => {

  return (
    <Modal show={isOpen} onClose={onClose} className="max-w-sm">
      <div className='flex flex-col items-center space-y-6 p-4'>
        <h3 className='text-lg font-bold text-slate-700 -mt-2 mb-0'>Register Identity to Vote</h3>
        <p className='text-sm text-slate-600 text-center'>
          {explanation || DEFAULT_EXPLANATION}
        </p>
        <button
          className={`button-primary w-full ${isRegistering ? 'button-disabled' : ''}`}
          disabled={isRegistering}
          onClick={onRegister}
        >
          {isRegistering ? 'Processing...' : 'Register Identity'}
        </button>
      </div>
    </Modal>
  );
};

export default RegistrationModal; 