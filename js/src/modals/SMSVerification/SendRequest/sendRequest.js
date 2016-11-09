// Copyright 2015, 2016 Ethcore (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

import React, { Component, PropTypes } from 'react';
import qs from 'querystring';

import TxHash from '../../../ui/TxHash';

import styles from './sendRequest.css';

const isValidReceipt = (receipt) => {
  return receipt && receipt.blockNumber && receipt.blockNumber.gt(0);
};

// TODO: DRY up with ../SendConfirmation
const waitForConfirmations = (api, tx, confirmations) => {
  return new Promise((resolve, reject) => {
    api.pollMethod('eth_getTransactionReceipt', tx, isValidReceipt)
    .then((receipt) => {
      let subscription;
      api.subscribe('eth_blockNumber', (err, block) => {
        if (err) {
          reject(err);
        } else if (block.minus(confirmations - 1).gte(receipt.blockNumber)) {
          api.unsubscribe(subscription);
          resolve();
        }
      })
      .then((_subscription) => {
        subscription = _subscription;
      })
      .catch(reject);
    });
  });
};

const postToVerificationServer = (query) => {
  query = qs.stringify(query);
  return fetch('https://sms-verification.parity.io/?' + query, {
    method: 'POST', mode: 'cors', cache: 'no-store'
  })
  .then((res) => {
    return res.json().then((data) => {
      if (res.ok) {
        return data.message;
      }
      throw new Error(data.message || 'unknown error');
    });
  });
};

export default class SendRequest extends Component {
  static contextTypes = {
    api: PropTypes.object.isRequired
  }

  static propTypes = {
    account: PropTypes.string.isRequired,
    contract: PropTypes.object.isRequired,
    data: PropTypes.object.isRequired,
    onData: PropTypes.func.isRequired,
    onSuccess: PropTypes.func.isRequired,
    onError: PropTypes.func.isRequired
  }

  state = {
    init: true,
    step: 'init'
  };

  componentWillMount () {
    const { init } = this.state;
    if (init) {
      this.send();
      this.setState({ init: false });
    }
  }

  render () {
    const { step } = this.state;

    if (step === 'error') {
      return (<p>{ this.state.error }</p>);
    }

    if (step === 'pending') {
      return (<p>Waiting for authorization by the Parity Signer.</p>);
    }

    if (step === 'posted') {
      return (
        <div className={ styles.centered }>
          <TxHash hash={ this.props.data.txHash } maxConfirmations={ 3 } />
          <p>Please keep this window open.</p>
        </div>);
    }

    if (step === 'mined') {
      return (<p>Requesting an SMS from the Parity server.</p>);
    }

    if (step === 'sms-sent') {
      return (<p>The verification code has been sent to { this.props.data.number }.</p>);
    }

    return null;
  }

  send = () => {
    const { api } = this.context;
    const { account, contract, onData, onError, onSuccess } = this.props;
    const { fee, number } = this.props.data;

    const request = contract.functions.find((fn) => fn.name === 'request');
    const options = { from: account, value: fee.toString() };

    request.estimateGas(options, [])
      .then((gas) => {
        options.gas = gas.mul(1.2).toFixed(0);
        // TODO: show message
        this.setState({ step: 'pending' });
        return request.postTransaction(options, []);
      })
      .then((handle) => {
        // TODO: The "request rejected" error doesn't have any property to
        // distinguish it from other errors, so we can't give a meaningful error here.
        return api.pollMethod('parity_checkRequest', handle);
      })
      .then((txHash) => {
        onData({ txHash: txHash });
        this.setState({ step: 'posted' });
        return waitForConfirmations(api, txHash, 3);
      })
      .then(() => {
        this.setState({ step: 'mined' });
        return postToVerificationServer({ number, address: account });
      })
      .then(() => {
        this.setState({ step: 'sms-sent' });
        onSuccess();
      })
      .catch((err) => {
        console.error('failed to request sms verification', err);
        onError(err);
        this.setState({
          step: 'error',
          error: 'Failed to request a confirmation SMS: ' + err.message
        });
        // TODO: show message in SnackBar
      });
  }
}
