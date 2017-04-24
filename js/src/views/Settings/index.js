// Copyright 2015-2017 Parity Technologies (UK) Ltd.
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

import ReactDOM from 'react-dom';
import React from 'react';
import { IndexRedirect, Route, Router, hashHistory } from 'react-router';

import injectTapEventPlugin from 'react-tap-event-plugin';
injectTapEventPlugin();

import SettingsBackground from './Background';
import SettingsParity from './Node';
import SettingsProxy from './Proxy';
import SettingsViews from './Views';
import Main from './main';

import '~/../assets/fonts/Roboto/font.css';
import '~/../assets/fonts/RobotoMono/font.css';

import './settings.css';

ReactDOM.render(
  <Router history={ hashHistory }>
    <Route path='/' component={ Main }>
      <Route path='/background' component={ SettingsBackground } />
      <Route path='/parity' component={ SettingsParity } />
      <Route path='/proxy' component={ SettingsProxy } />
      <Route path='/views' component={ SettingsViews } />
      <IndexRedirect to='/views' />
    </Route>
  </Router>,
  document.querySelector('#container')
);
